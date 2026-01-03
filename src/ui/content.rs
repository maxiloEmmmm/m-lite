use std::{borrow::Cow, sync::Arc, thread::sleep, time::Duration};

use chrono::{DateTime, NaiveDateTime};
use ratatui::{
    crossterm::{
        event::{Event, KeyCode},
        style::Colors,
    },
    layout::{Constraint, Layout},
    style::{Modifier, Style, Stylize, palette::tailwind::SLATE},
    text::Line,
    widgets::{
        Block, Borders, List, ListItem, ListState, Paragraph, StatefulWidgetRef, WidgetRef,
        block::Title,
    },
};
use tokio::io::join;

use crate::{
    event::{ES, Play, PlayListOP, PlayMode},
    m163::typ::{PlayDetail, PlayDetailInner, PlayItem, PlayList},
    play::PlayReq,
    ui::{
        app::{ShareCtx, global_help},
        focus::Focus,
        widgets::{help::Help, play_list::PlayList as PlayListWidget, tip::Msg},
    },
};

pub struct Content {
    list: Option<PlayDetail>,
    list_state: ListState,
    list_index: usize,
    focus: Focus,
    ctx: ShareCtx,
}

impl Content {
    pub fn new(ctx: ShareCtx, focus: Focus) -> Self {
        Content {
            list: None,
            list_state: ListState::default(),
            list_index: 0,
            focus: focus,
            ctx: ctx,
        }
    }
    fn init(&mut self) {
        if let Some(list) = self.list.as_ref() {
            if list.playlist.tracks.len() > 0 {
                self.list_state.select_first();
            }
        }
    }
    pub fn event(&mut self, e: &mut ES) {
        match e {
            ES::DataPlayListDetail(pl) => {
                self.list = Some(pl.clone());
                if self
                    .list
                    .as_ref()
                    .unwrap()
                    .playlist
                    .id
                    .eq(&self.ctx.borrow().like_play_id)
                    && self.list.as_ref().unwrap().playlist.ordered
                {
                    self.ctx.borrow_mut().like_set.clear();
                    for v in &self.list.as_ref().unwrap().playlist.tracks {
                        self.ctx.borrow_mut().like_set.insert(v.id);
                    }
                }
                self.init();
            }
            ES::DataRecommendSongs(ss) => {
                self.list = Some(PlayDetail {
                    playlist: PlayDetailInner {
                        id: 0,
                        name: "日推".to_owned(),
                        subscribed: false,
                        ordered: false,
                        creator: crate::m163::typ::PlayCreator {
                            nickname: "".to_owned(),
                            signature: "".to_owned(),
                            avatar_url: "".to_owned(),
                        },
                        description: None,
                        create_time: 0,
                        tags: vec![],
                        comment_count: 0,
                        play_count: 0,
                        cover_img_url: "".to_owned(),
                        tracks: ss
                            .recommend
                            .iter()
                            .map(|v| PlayItem {
                                name: v.name.to_owned(),
                                id: v.id,
                                dt: v.duration,
                                art_r: v.artists.clone(),
                            })
                            .collect(),
                    },
                });
                self.init();
            }
            ES::Event(ee) => {
                if !self.focus.is_me() {
                    return;
                }
                match ee {
                    Event::Key(ek) => match ek.code {
                        KeyCode::Char('h') => {
                            if self.focus.is_me() {
                                self.ctx.borrow_mut().add_modal(Help::new(global_help(vec![
                                    ("d".to_owned(), "从歌单移除歌曲".to_owned()),
                                    ("t".to_owned(), "加入到某个歌单".to_owned()),
                                    ("s".to_owned(), "收藏歌单".to_owned()),
                                    ("r".to_owned(), "随机播放歌单".to_owned()),
                                    ("o".to_owned(), "列表播放歌单".to_owned()),
                                    ("j/k".to_owned(), "下/上移动".to_owned()),
                                    ("esc".to_owned(), "返回上一级".to_owned()),
                                    ("enter".to_owned(), "播放".to_owned()),
                                    ("p".to_owned(), "打开播放列表".to_owned()),
                                ])));
                            }
                        }
                        KeyCode::Char('d') => {
                            if self.list_state.selected().is_none() {
                                return;
                            }
                            if let Some(play_list) = self.list.as_ref() {
                                let song =
                                    &play_list.playlist.tracks[self.list_state.selected().unwrap()];
                                let au = self.ctx.borrow().async_clone();
                                let id = song.id;
                                let pid = self.list.as_ref().unwrap().playlist.id;
                                self.ctx.borrow_mut().confirm(
                                    &format!(
                                        "确认从[{}]移除[{}]?",
                                        play_list.playlist.name.as_str(),
                                        song.name.as_str(),
                                    ),
                                    move |ok| {
                                        au.rt.spawn({
                                            let aux = au.clone();
                                            async move {
                                                match aux.nc.track(false, pid, vec![id]).await {
                                                    Ok(_) => {
                                                        aux.tx.send(ES::Tip(Msg(
                                                            "移除成功",
                                                            Duration::from_millis(1500),
                                                        )));
                                                        aux.nc.clear_play_list(pid);
                                                        match aux.nc.play_detail(pid).await {
                                                            Ok(d) => {
                                                                aux.tx.send(
                                                                    ES::DataPlayListDetail(d),
                                                                );
                                                            }
                                                            Err(e) => {
                                                                println!("e {}", e.to_string())
                                                            }
                                                        }
                                                    }
                                                    Err(e) => println!("e {}", e.to_string()),
                                                }
                                            }
                                        });
                                    },
                                );
                            }
                        }
                        KeyCode::Char('t') => {
                            if self.list_state.selected().is_none() {
                                return;
                            }
                            if let Some(play_list) = self.list.as_ref() {
                                let id = play_list.playlist.tracks
                                    [self.list_state.selected().unwrap()]
                                .id;
                                let au = self.ctx.borrow().async_clone();
                                self.ctx.borrow_mut().add_modal(PlayListWidget::new(
                                    "加入个人歌单列表",
                                    {
                                        move |item: &crate::m163::typ::PlayListItem| {
                                            au.rt.spawn({
                                                let aux = au.clone();
                                                let pid = item.id;
                                                async move {
                                                    match aux.nc.track(true, pid, vec![id]).await {
                                                        Ok(e) => {
                                                            aux.nc.clear_play_list(pid);
                                                            aux.tx.send(ES::Tip(Msg(
                                                                "收藏成功!",
                                                                Duration::from_millis(1500),
                                                            )));
                                                        }
                                                        Err(e) => println!("e {}", e.to_string()),
                                                    }
                                                }
                                            });
                                        }
                                    },
                                ));
                                self.ctx.borrow().rt.spawn({
                                    let txx = self.ctx.borrow().tx.clone();
                                    let ncx = self.ctx.borrow().nc.clone();
                                    async move {
                                        match ncx.play_list(0, 1000).await {
                                            Ok(d) => {
                                                txx.send(ES::DataPlayList(d));
                                            }
                                            Err(e) => {
                                                println!("e {}", e.to_string());
                                            }
                                        }
                                    }
                                });
                            }
                        }
                        KeyCode::Char('s') => {
                            if self.list.is_none() {
                                return;
                            }
                            if self.list.as_ref().unwrap().playlist.subscribed {
                                self.ctx
                                    .borrow()
                                    .tx
                                    .send(ES::Tip(Msg("已收藏!", Duration::from_secs(1))));
                                return;
                            }
                            self.ctx.borrow().rt.spawn({
                                let txx = self.ctx.borrow().tx.clone();
                                let ncx = self.ctx.borrow().nc.clone();
                                let id = self.list.as_ref().unwrap().playlist.id;
                                if id == 0 {
                                    // 暂不支持每日推荐新建歌单
                                    return;
                                }
                                async move {
                                    match ncx.sub_play(id).await {
                                        Ok(_) => {
                                            txx.send(ES::Tip(Msg(
                                                "收藏成功!",
                                                Duration::from_secs(2),
                                            )));
                                            ncx.clear_play();
                                        }
                                        Err(e) => println!("e {}", e.to_string()),
                                    }
                                }
                            });
                        }
                        KeyCode::Char('r') => {
                            if self.list.is_none() {
                                return;
                            }
                            self.ctx.borrow().tx.send(ES::Play(Play::PlayList((
                                self.list.clone().unwrap().playlist.tracks,
                                PlayListOP::Set,
                            ))));
                            self.ctx
                                .borrow()
                                .tx
                                .send(ES::Play(Play::PlayMode(PlayMode::Random)));
                            let index = fastrand::Rng::new()
                                .usize(0..self.list.as_ref().unwrap().playlist.tracks.len());

                            self.ctx.borrow().ptx.send(PlayReq::Play(
                                self.list.as_ref().unwrap().playlist.tracks[index].id,
                            ));
                        }
                        KeyCode::Char('o') => {
                            if self.list.is_none() {
                                return;
                            }
                            self.ctx.borrow().tx.send(ES::Play(Play::PlayList((
                                self.list.clone().unwrap().playlist.tracks,
                                PlayListOP::Set,
                            ))));
                            self.ctx
                                .borrow()
                                .tx
                                .send(ES::Play(Play::PlayMode(PlayMode::Order)));
                            self.ctx.borrow().ptx.send(PlayReq::Play(
                                self.list.as_ref().unwrap().playlist.tracks[0].id,
                            ));
                        }
                        KeyCode::Char('k') => {
                            self.list_state.select_previous();
                        }
                        KeyCode::Char('j') => self.list_state.select_next(),
                        KeyCode::Esc => self.focus.back(),
                        KeyCode::Enter => {
                            if self.list.is_none() {
                                return;
                            }
                            self.list_index = self.list_state.selected().unwrap_or(0);
                            let current = self.list.as_ref().unwrap().playlist.tracks
                                [self.list_state.selected().unwrap()]
                            .clone();
                            let id = current.id;
                            self.ctx.borrow().tx.send(ES::Play(Play::PlayList((
                                vec![current],
                                PlayListOP::Append,
                            ))));
                            self.ctx.borrow().ptx.send(PlayReq::Play(id));
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }

            _ => {}
        }
    }

    pub fn render_ref(&mut self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let mut b = Block::new().title(
            self.list
                .as_ref()
                .map(|v| v.playlist.name.as_str())
                .unwrap_or(""),
        );
        if self.focus.is_me() {
            b = b.borders(Borders::ALL);
        }
        let inner = b.inner(area);
        b.render_ref(area, buf);

        let mut layouts = Layout::vertical(vec![Constraint::Length(5), Constraint::Fill(1)])
            .spacing(1)
            .split(inner);
        match &self.list {
            Some(pl) => {
                Paragraph::new(format!(
                    r#"标签[{}] {}
作者:{} 签名:{}
{}
"#,
                    pl.playlist.tags.join(","),
                    DateTime::from_timestamp_millis(pl.playlist.create_time as i64)
                        .map(|v| v.format("%Y-%m-%d").to_string())
                        .unwrap_or("-".to_owned()),
                    pl.playlist.creator.nickname.as_str(),
                    pl.playlist.creator.signature.as_str(),
                    pl.playlist
                        .description
                        .as_ref()
                        .map(|v| v.as_str())
                        .unwrap_or(""),
                ))
                .block(Block::bordered().title("信息"))
                .render_ref(layouts[0], buf);
                StatefulWidgetRef::render_ref(
                    &(List::new(
                        pl.playlist
                            .tracks
                            .iter()
                            .enumerate()
                            .map(|(i, v)| {
                                let mut vv = Cow::Borrowed(v.name.as_str());
                                if self.list_index == i {
                                    vv = Cow::Owned(format!("*{}", vv));
                                }

                                if self.ctx.borrow().like_set.get(&v.id).is_some() {
                                    vv = Cow::Owned(format!("{} 💗", vv));
                                }

                                let mut style = Style::default();
                                if self.ctx.borrow().offline
                                    && self.ctx.borrow().nc.song_cached(v.id)
                                {
                                    style = style.red();
                                }
                                let line = Line::styled(vv, style);

                                ListItem::new(line)
                            })
                            .collect::<Vec<_>>(),
                    )
                    .highlight_style(Style::new().bg(SLATE.c800).add_modifier(Modifier::BOLD))
                    .highlight_symbol(">")),
                    layouts[1],
                    buf,
                    &mut self.list_state,
                );
            }
            None => "loading".render_ref(layouts[0], buf),
        }
    }
}
