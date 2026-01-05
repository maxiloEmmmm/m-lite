use std::{
    any::Any,
    collections::HashMap,
    fs::File,
    io::{self, Write},
    rc::Rc,
    str::FromStr,
    sync::{Arc, mpsc::Sender},
};

use aes::cipher::{
    BlockEncryptMut, BlockSizeUser, KeyIvInit,
    block_padding::{Padding, Pkcs7},
};
use base64::{Engine, prelude::BASE64_STANDARD};
use chrono::{Datelike, Local};
use color_eyre::owo_colors::OwoColorize;
use num_bigint::{BigInt, BigUint};
use num_traits::Num;
use rand::rngs::OsRng;
use reqwest::{
    Body, Url,
    cookie::{CookieStore, Jar},
    header::{
        ACCEPT, ACCEPT_ENCODING, ACCEPT_LANGUAGE, CONNECTION, CONTENT_TYPE, HOST, HeaderMap,
        HeaderName, HeaderValue, REFERER, SEC_WEBSOCKET_VERSION, USER_AGENT,
    },
};

use crate::{
    config::Config,
    event::ES,
    m163::{client::cache::COOKIE, typ},
    ui::widgets::tip::Msg,
};
use serde::Deserialize;
use serde_json::json;

const aes_key: [u8; 16] = [0x42; 16];

type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;
type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;

fn pkcs7_padded_len(len: usize, block: usize) -> usize {
    // 断言block_size是2的幂
    debug_assert!(block != 0 && (block & (block - 1)) == 0);
    (len + block) & !(block - 1)
}

fn rsa_no_padding(text: &[u8], pubkey: &str, modulus: &str) -> String {
    // 反转字节
    let mut bytes = text.to_vec();
    bytes.reverse();

    // 转成 hex，再转大整数
    let hex_str = hex::encode(bytes);
    let m = BigUint::from_str_radix(&hex_str, 16).unwrap();
    let e = BigUint::from_str_radix(pubkey, 16).unwrap();
    let n = BigUint::from_str_radix(modulus, 16).unwrap();

    // 幂模运算
    let c = m.modpow(&e, &n);

    // 补零输出（256字符，或根据 modulus 字节数决定）
    let k = ((n.bits() + 7) / 8) as usize;
    let mut out = format!("{:x}", c);
    while out.len() < k * 2 {
        out = format!("0{}", out);
    }

    out
}

#[derive(thiserror::Error, Debug)]
pub enum NCErr {
    #[error("any error")]
    Any,
    #[error("server respose {0}")]
    Resp(String),
    #[error("client {0}: {1}")]
    Client(String, String),
    #[error("offline")]
    Offline,
}

pub const TARGET: &str = "https://music.163.com";

mod cache {
    pub const COOKIE: &str = "cookie.cache";
    pub const PLAY_LIST: &str = "play_list";
}
pub struct Nc {
    client: reqwest::Client,
    down_client: reqwest::Client,
    // csrf: String,
    aes_1: Aes128CbcEnc,
    aes_2: Aes128CbcEnc,
    aes_key_rsa: String,
    _profile: tokio::sync::RwLock<Option<typ::Profile>>,
    jar: Arc<Jar>,
    url: Url,
    config: Config,
    event_tx: Sender<ES>,
}

impl Nc {
    pub fn new(event_tx: Sender<ES>, c: Config) -> Result<Nc, NCErr> {
        let mut cookie = c.cookie.to_owned();
        if cookie.is_empty() {
            cookie = match std::fs::read_to_string(c.Cache().join(cache::COOKIE)) {
                Ok(v) => v,
                Err(e) => {
                    if !e.kind().eq(&std::io::ErrorKind::NotFound) {
                        return Err(NCErr::Resp(format!("get cc {}", e.to_string())));
                    }
                    "".to_owned()
                }
            }
        }
        let mut client = reqwest::Client::builder();
        client = client.cookie_store(true);
        let jar = Arc::new(reqwest::cookie::Jar::default());
        // let mut csrf = String::default();
        let uu =
            Url::parse(TARGET).map_err(|err| NCErr::Client("url".to_owned(), err.to_string()))?;
        cookie.split(";").for_each(|v| {
            // if let Some((k, vv)) = v.trim().split_once("=") {
            //     if k.eq("__csrf") {
            //         println!("find csrf {}", vv);
            //         csrf = vv.to_owned();
            //     }
            // }

            jar.add_cookie_str(v, &uu);
        });
        client = client.cookie_provider(jar.clone());
        let mut down_client = reqwest::Client::builder()
            .cookie_store(true)
            .cookie_provider(jar.clone());
        let mut header = HeaderMap::new();
        header.insert(ACCEPT, HeaderValue::from_static("*/*"));
        header.insert(
            ACCEPT_ENCODING,
            HeaderValue::from_static("gzip,deflate,sdch"),
        );
        header.insert(
            ACCEPT_LANGUAGE,
            HeaderValue::from_static("zh-CN,zh;q=0.8,gl;q=0.6,zh-TW;q=0.4"),
        );
        header.insert(CONNECTION, HeaderValue::from_static("keep-alive"));
        header.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/x-www-form-urlencoded"),
        );
        header.insert(
            HOST,
            HeaderValue::from_static(&TARGET[TARGET.rfind('/').expect("invalid url") + 1..]),
        );
        header.insert(REFERER, HeaderValue::from_static(TARGET));
        header.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36"));
        let mut down_header = HeaderMap::new();
        down_header.insert(REFERER, HeaderValue::from_static(TARGET));
        down_header.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36"));
        let rnp = rsa_no_padding(
            aes_key.as_slice(),
            "010001",
            "00e0b509f6259df8642dbc35662901477df22677ec152b5ff68ace615bb7b725152b3ab17a876aea8a5aa76d2e417629ec4ee341f56135fccf695280104e0312ecbda92557c93870114af6c9d05c4f7f0c3685b7a46bee255932575cce10b424d813cfe4875d3e82047b97ddef52741d546b8e289dc6935b3ece0462db0a22b8e7",
        );
        Ok(Nc {
            url: uu,
            jar: jar.clone(),
            client: client
                .default_headers(header)
                .build()
                .map_err(|err| NCErr::Client("build".to_owned(), err.to_string()))?,
            down_client: down_client
                .default_headers(down_header)
                .build()
                .map_err(|err| NCErr::Client("down-build".to_owned(), err.to_string()))?,
            // csrf: csrf,
            aes_1: Aes128CbcEnc::new(
                "0CoJUm6Qyw8W8jud".as_bytes().into(),
                "0102030405060708".as_bytes().into(),
            ),
            aes_2: Aes128CbcEnc::new(&aes_key.into(), "0102030405060708".as_bytes().into()),
            aes_key_rsa: rnp,
            _profile: tokio::sync::RwLock::new(None),
            config: c,
            event_tx,
        })
    }

    pub async fn qr_wait_login(&self, key: &str, chain: &str) -> Result<typ::QRLogin, NCErr> {
        let req = self
            .client
            .post(format!("{}/weapi/login/qrcode/client/login", TARGET));
        // .header(
        //     HeaderName::from_static("x-login-chain-id"),
        //     HeaderValue::from_str(chain).unwrap(),
        // )
        // .header(
        //     HeaderName::from_static("x-loginmethod"),
        //     HeaderValue::from_static("QrCode"),
        // )
        // .header(
        //     HeaderName::from_static("x-os"),
        //     HeaderValue::from_static("web"),
        // )
        // .header(
        //     HeaderName::from_static("x-channelsource"),
        //     HeaderValue::from_static("undefined"),
        // );
        self._req(
            req,
            json!({
                "type": 1,
                "key": key.to_owned(),
                "noCheckToken": true
            }),
        )
        .await
        .map_err(|err| NCErr::Client("req".to_owned(), err.to_string()))
    }

    pub async fn qr_link(&self) -> Result<typ::QRR, NCErr> {
        let req = self
            .client
            .post(format!("{}/weapi/login/qrcode/unikey", TARGET));
        self._req(
            req,
            json!({
                "type": 1,
                "noCheckToken": true,
            }),
        )
        .await
        .map_err(|err| NCErr::Client("req".to_owned(), err.to_string()))
    }

    pub async fn logout(&self) {
        let req = self.client.post(format!("{}/weapi/logout", TARGET));
        self._req::<typ::Any>(req, json!({}))
            .await
            .map_err(|err| NCErr::Client("req".to_owned(), err.to_string()));
    }

    pub fn clear_cookie(&self) {
        std::fs::remove_file(self.config.Cache().join(COOKIE));
    }

    fn _cache<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<Option<T>, NCErr> {
        Ok(Some(
            serde_json::from_slice(&match std::fs::read(
                self.config.Cache().join(format!("{}.cache", key)),
            ) {
                Ok(data) => data,
                Err(e) => {
                    return if e.kind().eq(&io::ErrorKind::NotFound) {
                        Ok(None)
                    } else {
                        Err(NCErr::Resp(e.to_string()))
                    };
                }
            })
            .map_err(|e| NCErr::Resp(e.to_string()))?,
        ))
    }

    fn _set_cache<T: serde::Serialize>(&self, key: &str, v: &T) -> Result<(), NCErr> {
        let mut fd = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(self.config.Cache().join(format!("{}.cache", key)))
            .map_err(|e| NCErr::Resp(e.to_string()))?;
        fd.write_all(&serde_json::to_vec(v).map_err(|e| NCErr::Resp(e.to_string()))?)
            .map_err(|e| NCErr::Resp(e.to_string()))?;
        Ok(())
    }

    async fn _req<T: serde::de::DeserializeOwned>(
        &self,
        mut r: reqwest::RequestBuilder,
        mut data: serde_json::Value,
    ) -> Result<T, NCErr> {
        // if !self.csrf.is_empty() {
        //     match &mut data {
        //         serde_json::Value::Object(inner) => {
        //             inner.insert(
        //                 "csrf_token".to_owned(),
        //                 serde_json::Value::String(self.csrf.to_owned()),
        //             );
        //         }
        //         _ => return Err(NCErr::Client("not object".to_owned(), "".to_owned())),
        //     }
        // }
        let mut m1 = serde_json::to_vec(&data).map_err(|_| NCErr::Any)?;
        let mut mll = m1.len();
        m1.resize(pkcs7_padded_len(m1.len(), aes::Aes128::block_size()), 0);
        self.aes_1
            .clone()
            .encrypt_padded_mut::<Pkcs7>(&mut m1, mll)
            .map_err(|err| NCErr::Client("aes1".to_owned(), format!("{:?}jjjj", err).to_owned()))?;
        let mut buf = Vec::new();
        buf.resize(m1.len() * 4 / 3 + 4, 0);
        let size = BASE64_STANDARD
            .encode_slice(&m1, &mut buf)
            .map_err(|err| NCErr::Client("base1".to_owned(), err.to_string()))?;
        buf.truncate(size);
        let tmp = m1;
        m1 = buf;
        buf = tmp;
        mll = m1.len();
        m1.resize(pkcs7_padded_len(m1.len(), aes::Aes128::block_size()), 0);
        self.aes_2
            .clone()
            .encrypt_padded_mut::<Pkcs7>(&mut m1, mll)
            .map_err(|err| NCErr::Client("aes2".to_owned(), err.to_string()))?;
        buf.fill(0);
        buf.resize(m1.len() * 4 / 3 + 4, 0);
        let size = BASE64_STANDARD
            .encode_slice(m1, &mut buf)
            .map_err(|err| NCErr::Client("base2".to_owned(), err.to_string()))?;
        buf.truncate(size);
        r = r.form(&HashMap::from([
            ("params", unsafe {
                str::from_utf8_unchecked(buf.as_slice())
            }),
            ("encSecKey", self.aes_key_rsa.as_str()),
        ]));
        // if self.csrf.len() > 0 {
        //     r = r.query(&[("csrf_token", self.csrf.as_str())]);
        // }

        let resp = r.send().await.map_err(|err| {
            if err.is_timeout() || err.is_connect() {
                self.event_tx
                    .send(ES::AppState(crate::event::AppState::Offline));
                NCErr::Offline
            } else {
                NCErr::Client("req".to_owned(), err.to_string())
            }
        })?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|err| NCErr::Resp(err.to_string()))?;

        if !status.is_success() || text.len() == 0 {
            return Err(NCErr::Resp(text));
        }
        serde_json::from_str(text.as_str()).map_err(|err| {
            NCErr::Client(
                "resp".to_owned(),
                format!("{} err {}", text.as_str(), err.to_string()),
            )
        })
    }

    async fn _build(&self) -> Result<(), NCErr> {
        if self._profile.read().await.is_some() {
            return Ok(());
        }

        self._profile.write().await.replace(self.profile().await?);
        Ok(())
    }

    pub async fn recommend_resource(&self) -> Result<typ::RecommendPlayList, NCErr> {
        let now = Local::now();
        let key = &format!(
            "recommend_resource_{}-{:02}-{:02}",
            now.year(),
            now.month(),
            now.day()
        );
        if let Some(ret) = self._cache::<typ::RecommendPlayList>(key)? {
            return Ok(ret);
        }
        self._build().await?;

        let req = self
            .client
            .post(format!("{}/weapi/discovery/recommend/resource", TARGET));

        let ret = self
            ._req(req, json!({}))
            .await
            .map_err(|err| NCErr::Client("req".to_owned(), err.to_string()))?;
        self._set_cache(key, &ret)?;
        Ok(ret)
    }

    pub async fn recommend_songs(&self) -> Result<typ::MaybeRecommendSong, NCErr> {
        let now = Local::now();
        let key = &format!(
            "recommend_{}-{:02}-{:02}",
            now.year(),
            now.month(),
            now.day()
        );
        if let Some(ret) = self._cache::<typ::MaybeRecommendSong>(key)? {
            return Ok(ret);
        }
        self._build().await?;

        let req = self
            .client
            .post(format!("{}/weapi/v2/discovery/recommend/songs", TARGET));

        let ret = self
            ._req(
                req,
                json!({
                    "offset": 0,
                    "total": true,
                }),
            )
            .await
            .map_err(|err| NCErr::Client("req".to_owned(), err.to_string()))?;

        self._set_cache(key, &ret)?;
        Ok(ret)
    }

    pub async fn profile(&self) -> Result<typ::Profile, NCErr> {
        let req = self
            .client
            .post(format!("{}/weapi/w/nuser/account/get", TARGET));
        self._req(req, json!({})).await
    }

    pub fn save_cookie(&self) -> Result<(), NCErr> {
        let cookie = match self.jar.cookies(&self.url) {
            Some(v) => v,
            None => {
                return Ok(());
            }
        };

        let cs = cookie.to_str().unwrap();
        if !cs.is_empty() {
            let mut fd = std::fs::OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open(self.config.Cache().join(cache::COOKIE))
                .map_err(|e| NCErr::Resp(e.to_string()))?;
            fd.write_all(cs.as_bytes())
                .map_err(|e| NCErr::Resp(e.to_string()))?;
        }
        Ok(())
    }

    pub fn set_s_device_id(&self, id: &str) {
        self.jar
            .add_cookie_str(&format!("sDeviceId={}", id), &self.url);
    }

    pub fn s_device_id(&self) -> Option<String> {
        let cookie = match self.jar.cookies(&self.url) {
            Some(v) => v,
            None => {
                return None;
            }
        };

        let cs = cookie.to_str().unwrap();
        for v in cs.split(";") {
            let vv = v.split("=").collect::<Vec<_>>();
            if vv[0].eq("sDeviceId") {
                return Some(vv[1].to_owned());
            }
        }
        None
    }

    pub fn clear_play_list(&self, id: usize) -> Result<(), NCErr> {
        self._clear_cache(&format!("play_detail_{}", id))
    }
    pub fn _clear_cache(&self, key: &str) -> Result<(), NCErr> {
        std::fs::remove_file(self.config.Cache().join(format!("{}.cache", key)))
            .map_err(|e| NCErr::Resp(e.to_string()))
    }

    pub fn clear_play(&self) -> Result<(), NCErr> {
        self._clear_cache(cache::PLAY_LIST)
    }
    pub async fn search(&self, search: &str) -> Result<typ::SearchResult, NCErr> {
        self._build().await?;
        let req = self
            .client
            .post(format!("{}/weapi/cloudsearch/get/web", TARGET));

        self._req(
            req,
            json!({
                "s": search,
                "offset": "0",
                "limit": "10",
                "type": "1",
            }),
        )
        .await
        .map_err(|err| NCErr::Client("req".to_owned(), err.to_string()))
    }
    pub async fn lyric(&self, id: usize) -> Result<typ::Lyric, NCErr> {
        let key = &format!("{}.lyric", id);
        if let Some(ret) = self._cache::<typ::Lyric>(key)? {
            return Ok(ret);
        }
        self._build().await?;
        let req = self.client.post(format!("{}/weapi/song/lyric", TARGET));

        let ret = self
            ._req(
                req,
                json!({
                    "id": id.to_string(),
                    "tv": -1,
                    "lv": -1,
                }),
            )
            .await
            .map_err(|err| NCErr::Client("req".to_owned(), err.to_string()))?;

        self._set_cache(key, &ret)?;
        Ok(ret)
    }

    pub async fn create_play_list(&self, name: &str) -> Result<typ::Any, NCErr> {
        self._build().await?;
        let req = self
            .client
            .post(format!("{}/weapi/playlist/create", TARGET));

        self._req(
            req,
            json!({
                "name": name,
            }),
        )
        .await
        .map_err(|err| NCErr::Client("req".to_owned(), err.to_string()))
    }
    pub async fn delete_play_list(&self, id: usize) -> Result<typ::Any, NCErr> {
        self._build().await?;
        let req = self
            .client
            .post(format!("{}/weapi/playlist/delete", TARGET));

        self._req(
            req,
            json!({
                "pid": id.to_string(),
            }),
        )
        .await
        .map_err(|err| NCErr::Client("req".to_owned(), err.to_string()))
    }

    pub async fn track(
        &self,
        add: bool,
        play_id: usize,
        songs: Vec<usize>,
    ) -> Result<typ::Any, NCErr> {
        self._build().await?;
        let req = self
            .client
            .post(format!("{}/weapi/playlist/manipulate/tracks", TARGET));

        self._req(
            req,
            json!({
                "trackIds": format!("[{}]", songs.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(",")),
                "pid": play_id.to_string(),
                "op": if add { "add" } else { "del" },
            }),
        )
        .await
        .map_err(|err| NCErr::Client("req".to_owned(), err.to_string()))
    }

    pub async fn play_list(&self, offset: usize, limit: usize) -> Result<typ::PlayList, NCErr> {
        if let Some(ret) = self._cache::<typ::PlayList>(cache::PLAY_LIST)? {
            return Ok(ret);
        }
        self._build().await?;

        let req = self.client.post(format!("{}/weapi/user/playlist", TARGET));

        let ret = self
            ._req(
                req,
                json!({
                    "uid": self._profile.read().await.as_ref().unwrap().account.id,
                    "offset": offset,
                    "limit": limit,
                }),
            )
            .await
            .map_err(|err| NCErr::Client("req".to_owned(), err.to_string()))?;
        self._set_cache(cache::PLAY_LIST, &ret)?;
        Ok(ret)
    }

    pub async fn sub_play(&self, id: usize) -> Result<typ::Any, NCErr> {
        self._build().await?;
        let req = self
            .client
            .post(format!("{}/weapi/playlist/subscribe", TARGET));

        self._req(
            req,
            json!({
                "id": id,
            }),
        )
        .await
        .map_err(|err| NCErr::Client("req".to_owned(), err.to_string()))
    }

    pub async fn unsub_play(&self, id: usize) -> Result<typ::Any, NCErr> {
        self._build().await?;
        let req = self
            .client
            .post(format!("{}/weapi/playlist/unsubscribe", TARGET));

        self._req(
            req,
            json!({
                "id": id,
            }),
        )
        .await
        .map_err(|err| NCErr::Client("req".to_owned(), err.to_string()))
    }
    pub async fn play_detail(&self, id: usize) -> Result<typ::PlayDetail, NCErr> {
        let key = &format!("play_detail_{}", id);
        if let Some(ret) = self._cache::<typ::PlayDetail>(key)? {
            return Ok(ret);
        }
        self._build().await?;

        let req = self
            .client
            .post(format!("{}/weapi/v6/playlist/detail", TARGET));

        let ret = self
            ._req(
                req,
                json!({
                    "id": id,
                    "total": true,
                    "n": 1000,
                    "limit": 1000,
                    "offset": 0,
                }),
            )
            .await
            .map_err(|err| NCErr::Client("req".to_owned(), err.to_string()))?;
        self._set_cache(key, &ret)?;
        Ok(ret)
    }

    pub async fn song_url(&self, id: usize) -> Result<typ::SongUrl, NCErr> {
        self._build().await?;

        let req = self
            .client
            .post(format!("{}/weapi/song/enhance/player/url", TARGET));

        self._req(
            req,
            json!({
                "ids": format!("[{}]", id),
                "br": 128000,
            }),
        )
        .await
        .map_err(|err| NCErr::Client("req".to_owned(), err.to_string()))
    }

    pub async fn song(&self, id: usize) -> Result<typ::Song, NCErr> {
        self._build().await?;

        let req = self.client.post(format!("{}/weapi/song/detail", TARGET));

        self._req(
            req,
            json!({
                "ids": format!("[{}]", id),
            }),
        )
        .await
        .map_err(|err| NCErr::Client("req".to_owned(), err.to_string()))
    }

    pub fn song_cached(&self, id: usize) -> bool {
        let path = self.config.Cache().join(format!("{}.mp3", id));
        std::fs::metadata(&path).is_ok()
    }

    pub async fn download(&self, url: &str, id: usize) -> Result<(), NCErr> {
        let path = self.config.Cache().join(format!("{}.mp3", id));
        if let Ok(_) = std::fs::metadata(&path) {
            return Ok(());
        }
        self._build().await?;
        let mut resp = match self.down_client.get(url).send().await {
            Ok(r) => r,
            Err(e) => {
                if e.is_timeout() || e.is_connect() {
                    self.event_tx
                        .send(ES::AppState(crate::event::AppState::Offline));
                }
                return Err(NCErr::Resp(format!("resp {}", e)));
            }
        };

        if !resp.status().is_success() {
            return Err(NCErr::Resp(format!(
                "resp1 status {}",
                resp.status().as_str()
            )));
        }
        let mut file = match File::create(&path) {
            Ok(file) => file,
            Err(e) => {
                return Err(NCErr::Resp(format!("resp2 {}", e)));
            }
        };
        let bytes = match resp.bytes().await {
            Ok(r) => r,
            Err(e) => {
                return Err(NCErr::Resp(format!("resp3 {}", e)));
            }
        }; // 一次性读取全部内容
        match file.write_all(&bytes) {
            Ok(r) => {}
            Err(e) => {
                return Err(NCErr::Resp(format!("resp4 {}", e)));
            }
        };
        Ok(())
    }
}
