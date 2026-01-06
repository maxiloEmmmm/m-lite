use std::{
    fmt::{self, Debug},
    ops::Sub,
    sync::{Arc, mpsc::Sender},
    time::Duration,
};

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent};
use tokio_util::sync::CancellationToken;

use crate::{
    m163::typ::{
        MaybeRecommendSong, PlayDetail, PlayItem, PlayList, Profile, RecommendPlayList,
        SearchResult,
    },
    ui::{footer::Lyric, widgets::tip::Tip},
};

pub async fn ui_event_loop(event_tx: Sender<ES>, cancel: CancellationToken) {
    loop {
        match event::poll(Duration::from_millis(250)) {
            Ok(has) => {
                if has {
                    match event::read() {
                        Ok(ee) => {
                            event_tx.send(ES::Event(ee));
                        }
                        Err(e) => {
                            println!("read event {:?}", e);
                        }
                    }
                }
            }
            Err(e) => {
                println!("poll event {:?}", e);
            }
        }
        if cancel.is_cancelled() {
            return;
        }
    }
}

#[derive(Debug)]
pub enum ES {
    DataSearch(SearchResult),
    DataProfile(Profile),
    DataPlayList(PlayList),
    DataRecommendResource(RecommendPlayList),
    DataRecommendSongs(MaybeRecommendSong),
    DataPlayListDetail(PlayDetail),
    RuntimeHead(HeadMenuKey),
    Event(Event),
    Render,
    Play(Play),
    SEC,
    LoginLink(String),
    LoginState(LoginState),
    AppState(AppState),
    Tip(Tip),
}

#[derive(Debug, Clone)]
pub enum LoginState {
    Wait,
    Authing,
    Failed,
    Ok,
}

#[derive(Debug, Clone)]
pub enum AppState {
    Authing,
    Authed,
    Offline,
}

#[derive(Debug, Clone)]
pub enum HeadMenuKey {
    My,
    Maybe,
    About,
}

#[derive(Debug, Clone)]
pub enum PlayMode {
    Order,
    Single,
    SingleLoop,
    Random,
}

#[derive(Debug, Clone)]
pub enum Play {
    State(PlayState),
    Offset(Duration),
    Lyric(Vec<Lyric>),
    PlayList((Vec<PlayItem>, PlayListOP)),
    PlayMode(PlayMode),
}

#[derive(Debug, Clone)]
pub enum PlayListOP {
    Set,
    Append,
}

#[derive(Debug, Clone)]
pub enum PlayState {
    Play(usize, String),
    Start,
    Stop,
    None,
    Failed(usize),
}
