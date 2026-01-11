use std::{
    fs::OpenOptions,
    rc::Rc,
    sync::{Arc, mpsc::Sender},
    time::Duration,
};

use rodio::Sink;
use tokio::{
    select,
    sync::mpsc::{self, UnboundedSender},
};
use tokio_util::sync::CancellationToken;

use crate::{
    config::Config,
    event::{AppState, ES, LoginState, Play, PlayState},
    m163::client::Nc,
    ui::{
        app::Wrap,
        widgets::tip::{Msg, SimpleMsg},
    },
};

#[derive(Debug, Clone)]
pub enum PlayReq {
    Play(usize),
    Start,
    Stop,
    Login,
    WatchLogin(String, String),
    T(Duration),
    V(f32),
}

pub struct PlayCtx {
    pub nc: Arc<Nc>,
    pub event_tx: Sender<ES>,
    pub cancel: CancellationToken,
    pub config: Config,
}

pub fn play(mut ctx: PlayCtx, sink: Sink) -> (impl Future<Output = ()>, UnboundedSender<PlayReq>) {
    // todo 为什么返回impl Future, 就不行呢
    let (tx, mut rx) = mpsc::unbounded_channel();
    (
        async move {
            let mut tick = tokio::time::interval(Duration::from_secs(1));
            loop {
                select! {
                    req = rx.recv() => {
                        if let Some(req) = req {
                            match req {
                                PlayReq::Login => {
                                    let link = match ctx.nc.qr_link().await {
                                        Ok(d) => d,
                                        Err(e) => {
                                            ctx.event_tx.send(ES::Tip(Msg(&format!("qr link req {}", e.to_string()), Duration::from_secs(2))));
                                            continue;
                                        }
                                    };
                                    ctx.event_tx.send(ES::LoginLink(link.unikey));
                                },
                                PlayReq::WatchLogin(key, chain) => {
                                    let result = ctx.nc.qr_wait_login(key.as_str(), chain.as_str()).await.unwrap();
                                    match result.code {
                                        800 => {
                                            ctx.event_tx.send(ES::LoginState(LoginState::Failed));
                                        },
                                        801 => {
                                            ctx.event_tx.send(ES::LoginState(LoginState::Wait));
                                        },
                                        802 => {
                                            ctx.event_tx.send(ES::LoginState(LoginState::Authing));
                                        },
                                        803 => {
                                            if let Err(e) = ctx.nc.save_cookie() {
                                                panic!("save {}", e.to_string());
                                            };
                                            ctx.event_tx.send(ES::AppState(AppState::Authed));
                                            ctx.event_tx.send(ES::LoginState(LoginState::Ok));
                                        },
                                        other => {
                                            ctx.event_tx.send(ES::Tip(Msg(&format!("watch log unexpected {}", other), Duration::from_secs(2))));
                                        },
                                    }
                                },
                                PlayReq::Stop => {
                                    sink.pause();
                                    ctx.event_tx.send(ES::Play(Play::State(PlayState::Stop)));
                                },
                                PlayReq::Start => {
                                    sink.play();
                                    ctx.event_tx.send(ES::Play(Play::State(PlayState::Start)));
                                },
                                PlayReq::T(v) => {
                                    sink.try_seek(v);
                                }
                                PlayReq::V(v) => {
                                    sink.set_volume(v);
                                    ctx.config.volume = v;
                                    ctx.config.save();
                                }
                                PlayReq::Play(id) => {
                                    let lyric = match ctx.nc.lyric(id).await {
                                        Ok(d) => d,
                                        Err(e) => {
                                            ctx.event_tx.wrap_error("lyric", &e);
                                            ctx.event_tx.send(ES::Play(Play::State(PlayState::Failed(id))));
                                            continue;
                                        }
                                    };
                                    if !ctx.nc.song_cached(id) {
                                        match ctx.nc.song_url(id).await {
                                            Ok(song) => {
                                                ctx.event_tx.send(ES::Tip(SimpleMsg("start down", Duration::from_secs(1))));
                                                match ctx.nc
                                                    .download(
                                                        song.data[0].url.as_str(),
                                                        id,
                                                    )
                                                    .await
                                                {
                                                    Ok(r) => r,
                                                    Err(e) => {
                                                        ctx.event_tx.wrap_error("download", &e);
                                                        ctx.event_tx.send(ES::Play(Play::State(PlayState::Failed(id))));
                                                        continue;
                                                    }
                                                };
                                                ctx.event_tx.send(ES::Tip(SimpleMsg("down ok", Duration::from_secs(1))));

                                            }
                                            Err(e) => {
                                                ctx.event_tx.wrap_error("req.song.url", &e);
                                                ctx.event_tx.send(ES::Play(Play::State(PlayState::Failed(id))));
                                                continue;
                                            }
                                        }
                                    }

                                    let file = match OpenOptions::new()
                                            .read(true)
                                            .open(ctx.config.Cache().join(format!("{}.mp3", id)))
                                        {
                                            Ok(file) => file,
                                            Err(e) => {
                                                ctx.event_tx.wrap_error("load.song_file", &e);
                                                ctx.event_tx.send(ES::Play(Play::State(PlayState::Failed(id))));
                                                return;
                                            }
                                        };
                                        sink.clear();
                                        sink.append(rodio::Decoder::try_from(file).unwrap());
                                        ctx.event_tx.send(ES::Play(Play::State(PlayState::Play(id, lyric.lrc.lyric))));
                                        ctx.event_tx.send(ES::Play(Play::State(PlayState::Start)));
                                        sink.play();
                                }
                                _ => {},
                            }
                         }
                    }
                    _ = tick.tick() => {
                        ctx.event_tx.send(ES::SEC);
                    }
                    _ = ctx.cancel.cancelled() => {
                        return;
                    }
                };
            }
        },
        tx,
    )
}
