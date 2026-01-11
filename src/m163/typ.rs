use std::{default, num::Saturating};

use serde::{Deserialize, Serialize, de::value::EnumAccessDeserializer};

#[derive(Debug, Deserialize)]
pub struct Any {
    pub code: i32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Lyric {
    pub code: i32,
    pub lrc: LyricInner,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LyricInner {
    pub lyric: String,
}

#[derive(Debug, Deserialize)]
pub struct QRLogin {
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct SearchResult {
    pub code: i32,
    pub result: SearchResultInner,
}

#[derive(Debug, Deserialize)]
pub struct SearchResultInner {
    #[serde(rename = "songCount")]
    pub song_count: u32,
    #[serde(default)]
    pub songs: Vec<PlayItem>,
}

#[derive(Debug, Deserialize)]
pub struct QRR {
    pub code: i32,
    pub unikey: String,
}

#[derive(Debug, Deserialize)]
pub struct Profile {
    pub code: i32,
    pub account: Account,
    pub profile: ProfileInner,
}

#[derive(Debug, Deserialize)]
pub struct Account {
    pub id: u64,
    #[serde(rename = "userName")]
    pub user_name: String,
    // #[serde(rename = "type")]
    // pub account_type: i32,
    // pub status: i32,
    // #[serde(rename = "whitelistAuthority")]
    // pub whitelist_authority: i32,
    // #[serde(rename = "createTime")]
    // pub create_time: i64,
    // #[serde(rename = "tokenVersion")]
    // pub token_version: i32,
    // pub ban: i32,
    // #[serde(rename = "baoyueVersion")]
    // pub baoyue_version: i32,
    // #[serde(rename = "donateVersion")]
    // pub donate_version: i32,
    // #[serde(rename = "vipType")]
    // pub vip_type: i32,
    // #[serde(rename = "anonimousUser")]
    // pub anonimous_user: bool,
    // #[serde(rename = "paidFee")]
    // pub paid_fee: bool,
}

#[derive(Debug, Deserialize)]
pub struct ProfileInner {
    // #[serde(rename = "userId")]
    // pub user_id: u64,
    // #[serde(rename = "userType")]
    // pub user_type: i32,
    pub nickname: String,
    // #[serde(rename = "avatarImgId")]
    // pub avatar_img_id: i64,
    // #[serde(rename = "avatarUrl")]
    // pub avatar_url: String,
    // #[serde(rename = "backgroundImgId")]
    // pub background_img_id: i64,
    // #[serde(rename = "backgroundUrl")]
    // pub background_url: String,
    pub signature: Option<String>,
    // #[serde(rename = "createTime")]
    // pub create_time: i64,
    // #[serde(rename = "userName")]
    // pub user_name: String,
    // #[serde(rename = "accountType")]
    // pub account_type: i32,
    // #[serde(rename = "shortUserName")]
    // pub short_user_name: String,
    // pub birthday: i64,
    // pub authority: i32,
    // pub gender: i32,
    // #[serde(rename = "accountStatus")]
    // pub account_status: i32,
    // pub province: i32,
    // pub city: i32,
    // #[serde(rename = "authStatus")]
    // pub auth_status: i32,
    // pub description: Option<String>,
    // #[serde(rename = "detailDescription")]
    // pub detail_description: Option<String>,
    // #[serde(rename = "defaultAvatar")]
    // pub default_avatar: bool,
    // #[serde(rename = "expertTags")]
    // pub expert_tags: Option<Vec<String>>,
    // pub experts: Option<serde_json::Value>,
    // #[serde(rename = "djStatus")]
    // pub dj_status: i32,
    // #[serde(rename = "locationStatus")]
    // pub location_status: i32,
    // #[serde(rename = "vipType")]
    // pub vip_type: i32,
    // pub followed: bool,
    // pub mutual: bool,
    // pub authenticated: bool,
    // #[serde(rename = "lastLoginTime")]
    // pub last_login_time: i64,
    // #[serde(rename = "lastLoginIP")]
    // pub last_login_ip: String,
    // #[serde(rename = "remarkName")]
    // pub remark_name: Option<String>,
    // #[serde(rename = "viptypeVersion")]
    // pub viptype_version: i64,
    // #[serde(rename = "authenticationTypes")]
    // pub authentication_types: i32,
    // #[serde(rename = "avatarDetail")]
    // pub avatar_detail: Option<serde_json::Value>,
    // pub anchor: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlayList {
    pub more: bool,
    #[serde(rename = "playlist")]
    pub list: Vec<PlayListItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlayListItem {
    pub id: usize,
    /// 收藏与否
    pub subscribed: bool,
    pub name: String,
    /// 封面地址
    #[serde(rename = "coverImgUrl")]
    pub cover_img_url: String,
    /// 歌曲个数
    #[serde(rename = "trackCount")]
    pub track_count: usize,
    /// 播放次数
    #[serde(rename = "playCount")]
    pub play_count: usize,
    ///home true 为喜欢列表
    pub ordered: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RecommendPlayList {
    pub recommend: Vec<RecommendPlayListItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RecommendPlayListItem {
    pub id: usize,
    #[serde(rename = "picUrl")]
    pub pic_url: String,
    pub name: String,
    #[serde(rename = "playcount")]
    pub play_count: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlayDetail {
    pub playlist: PlayDetailInner,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlayDetailInner {
    #[serde(default)]
    pub id: usize,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub subscribed: bool,
    #[serde(default)]
    pub ordered: bool,
    #[serde(default, rename = "coverImgUrl")]
    pub cover_img_url: String,
    pub tags: Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "createTime", default)]
    pub create_time: u64, // 13
    #[serde(rename = "commentCount", default)]
    pub comment_count: u32,
    #[serde(rename = "playCount", default)]
    pub play_count: u64,
    #[serde(default)]
    pub creator: PlayCreator,
    /// 收录歌曲
    pub tracks: Vec<PlayItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct PlayCreator {
    #[serde(default)]
    pub nickname: String,
    #[serde(default)]
    pub signature: String,
    #[serde(rename = "avatarUrl", default)]
    pub avatar_url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlayItem {
    pub name: String,
    pub id: usize,
    pub dt: u64,
    // // pub copyright: MusicCopyright,
    // /// 作者列表
    #[serde(rename = "ar")]
    pub art_r: Vec<Arter>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MusicCopyright {
    Yes = 0,
    NO = 1,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Arter {
    pub id: usize,
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SongUrl {
    pub data: Vec<SongUrlItem>,
}

#[derive(Debug, Deserialize)]
pub struct SongUrlItem {
    pub url: String,
    pub time: usize,
}

#[derive(Debug, Deserialize)]
pub struct Song {
    #[serde(rename = "songs")]
    pub data: Vec<SongItem>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SongItem {
    pub id: usize,
    pub name: String,
    pub artists: Vec<Arter>,
    pub duration: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MaybeRecommendSong {
    pub recommend: Vec<SongItem>,
}
