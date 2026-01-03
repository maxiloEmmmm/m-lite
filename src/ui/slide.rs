use std::{sync::Arc, time::Duration};

use ratatui::{
    crossterm::event::{Event, KeyCode, KeyModifiers},
    style::{Modifier, Style, palette::tailwind::SLATE},
    text::Line,
    widgets::{Block, Borders, List, ListItem, ListState, StatefulWidgetRef, Widget, WidgetRef},
};
use rodio::cpal::FromSample;

use crate::{
    event::{ES, HeadMenuKey},
    m163::typ::{PlayList, PlayListItem},
    ui::{
        app::{ShareCtx, global_help},
        focus::Focus,
        widgets::{help::Help, input::Input, tip::Msg},
    },
};

pub struct Slide {
    list: Option<PlayList>,
    list_state: ListState,
    list_index: usize,
    focus: Focus,
    ctx: ShareCtx,
    runtime_head: HeadMenuKey,
}

impl Slide {
    pub fn new(ctx: ShareCtx, focus: Focus) -> Self {
        Slide {
            list: None,
            list_state: ListState::default(),
            list_index: 0,
            focus: focus,
            ctx: ctx,
            runtime_head: HeadMenuKey::My,
        }
    }
    fn init(&mut self) {
        if let Some(list) = self.list.as_ref() {
            if list.list.len() > 0 {
                self.list_state.select_first();
                self.load_list();
            }
        }
    }

    fn format_number(&self, num: usize) -> String {
        match num {
            0..=999 => num.to_string(),
            1000..=9_999 => format!("{:.1}k", num as f64 / 1000.0),
            10_000..=99_999 => format!("{:.1}w", num as f64 / 10_000.0),
            100_000..=9_999_999 => format!("{:.0}w", num as f64 / 10_000.0),
            10_000_000..=99_999_999 => format!("{:.0}w", num as f64 / 10_000.0),
            100_000_000..=999_999_999 => format!("{:.1}m", num as f64 / 100_000_000.0),
            _ => format!("{:.0}m", num as f64 / 100_000_000.0),
        }
        .trim_end_matches(".0")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
    }

    fn load_list(&mut self) {
        if let Some(index) = self.list_state.selected() {
            self.ctx.borrow().rt.spawn({
                let nc = self.ctx.borrow().nc.clone();
                let tx = self.ctx.borrow().tx.clone();
                let id = self.list.as_ref().unwrap().list[index].id;
                async move {
                    if id == 0 {
                        let resp = nc.recommend_songs().await;
                        match resp {
                            Ok(list) => {
                                tx.send(ES::DataRecommendSongs(list));
                            }
                            Err(e) => {
                                println!("load detail {}", e);
                            }
                        }
                    } else {
                        let resp = nc.play_detail(id).await;
                        match resp {
                            Ok(list) => {
                                tx.send(ES::DataPlayListDetail(list));
                            }
                            Err(e) => {
                                println!("load detail {}", e);
                            }
                        }
                    }
                }
            });
        }
    }

    pub fn event(&mut self, e: &mut ES) -> bool {
        match e {
            ES::DataPlayList(pl) => {
                if self.focus.is_me() {
                    self.list = Some(pl.clone());
                    self.ctx.borrow_mut().like_play_id = self.list.as_ref().unwrap().list[0].id;
                    self.init();
                }
            }

            ES::DataRecommendResource(s) => {
                let mut list: Vec<PlayListItem> = vec![PlayListItem {
                    id: 0,
                    subscribed: false,
                    name: String::from("喜欢"),
                    cover_img_url: String::from(""),
                    track_count: 0,
                    play_count: 0,
                    ordered: true,
                }];
                s.recommend.iter().for_each(|v| {
                    list.push(PlayListItem {
                        id: v.id,
                        subscribed: false,
                        name: v.name.to_owned(),
                        cover_img_url: v.pic_url.to_owned(),
                        track_count: 0,
                        play_count: v.play_count,
                        ordered: false,
                    });
                });
                self.list = Some(PlayList { more: false, list });
                self.init();
            }
            ES::RuntimeHead(rh) => {
                self.runtime_head = rh.clone();
            }
            ES::Event(ee) => {
                if self.focus.is_me() {
                    match ee {
                        Event::Key(ek) => {
                            match ek.code {
                                KeyCode::Char('h') => {
                                    self.ctx.borrow_mut().add_modal(Help::new(global_help(vec![
                                        ("n".to_owned(), "新建歌单".to_owned()),
                                        ("x".to_owned(), "删除歌单".to_owned()),
                                        ("d".to_owned(), "取消收藏歌单".to_owned()),
                                        ("x".to_owned(), "删除歌单".to_owned()),
                                        ("n".to_owned(), "创建歌单".to_owned()),
                                        ("c".to_owned(), "清理歌单缓存".to_owned()),
                                        ("j/k".to_owned(), "下/上移动".to_owned()),
                                        ("esc".to_owned(), "返回上一级".to_owned()),
                                        ("enter".to_owned(), "进入歌单".to_owned()),
                                        ("p".to_owned(), "打开播放列表".to_owned()),
                                    ])));
                                }
                                KeyCode::Char('n') => {
                                    let ctx = self.ctx.borrow().async_clone();
                                    self.ctx.borrow_mut().add_modal(Input::new(
                                        self.ctx.clone(),
                                        "创建歌单",
                                        move |v: String| {
                                            if !v.is_empty() {
                                                ctx.rt.spawn({
                                                    let ctx = ctx.clone();
                                                    async move {
                                                        match ctx
                                                            .nc
                                                            .create_play_list(v.as_str())
                                                            .await
                                                        {
                                                            Ok(_) => {
                                                                ctx.nc.clear_play();

                                                                ctx.tx.send(ES::Tip(Msg(
                                                                    "创建成功",
                                                                    Duration::from_millis(1500),
                                                                )));

                                                                match ctx
                                                                    .nc
                                                                    .play_list(0, 1000)
                                                                    .await
                                                                {
                                                                    Ok(d) => {
                                                                        ctx.tx.send(
                                                                            ES::DataPlayList(d),
                                                                        );
                                                                    }
                                                                    Err(e) => println!(
                                                                        "e {}",
                                                                        e.to_string()
                                                                    ),
                                                                };
                                                            }
                                                            Err(e) => {
                                                                println!("e {}", e.to_string())
                                                            }
                                                        }
                                                    }
                                                });
                                            }
                                        },
                                    ));
                                }
                                KeyCode::Char('x') => {
                                    if let Some(index) = self.list_state.selected() {
                                        if !self.list.as_ref().unwrap().list[index].subscribed {
                                            let ncx = self.ctx.borrow().nc.clone();
                                            let txx = self.ctx.borrow().tx.clone();
                                            let rtx = self.ctx.borrow().rt.clone();
                                            let id = self.list.as_ref().unwrap().list[index].id;
                                            self.ctx.borrow_mut().confirm(&format!(
                                                    "确认删除歌单 {}?",
                                                    self.list.as_ref().unwrap().list[index].name
                                                ),
                                                move |yes| {
                                                    if yes {
                                                        rtx.spawn({
                                                            let t2x = txx.clone();
                                                            let n2x = ncx.clone();
                                                            async move {
                                                                match n2x.delete_play_list(id).await {
                                                                    Ok(_) => {
                                                                        t2x.send(ES::Tip(Msg("删除歌单成功!", Duration::from_secs(2))));
                                                                        n2x.clear_play();
                                                                        match n2x.play_list(0, 1000).await {
                                                                            Ok(d) => {
                                                                                t2x.send(ES::DataPlayList(
                                                                                    d,
                                                                                ));
                                                                            }
                                                                            Err(e) => println!(
                                                                                "e {}",
                                                                                e.to_string()
                                                                            ),
                                                                        }
                                                                    }
                                                                    Err(e) => {
                                                                        println!("e {}", e.to_string());
                                                                    }
                                                                }
                                                            }
                                                        });
                                                    }
                                                },
                                            );
                                        }
                                    }
                                }
                                KeyCode::Char('d') => {
                                    if let Some(index) = self.list_state.selected() {
                                        if self.list.as_ref().unwrap().list[index].subscribed {
                                            let ncx = self.ctx.borrow().nc.clone();
                                            let txx = self.ctx.borrow().tx.clone();
                                            let rtx = self.ctx.borrow().rt.clone();
                                            let id = self.list.as_ref().unwrap().list[index].id;
                                            self.ctx.borrow_mut().confirm(&format!(
                                                    "确认取消收藏 {}?",
                                                    self.list.as_ref().unwrap().list[index].name
                                                ),
                                                move |yes| {
                                                    if yes {
                                                        rtx.spawn({
                                                            let t2x = txx.clone();
                                                            let n2x = ncx.clone();
                                                            async move {
                                                                match n2x.unsub_play(id).await {
                                                                    Ok(_) => {
                                                                        t2x.send(ES::Tip(Msg("取消收藏成功!", Duration::from_secs(2))));
                                                                        n2x.clear_play();
                                                                        match n2x.play_list(0, 1000).await {
                                                                            Ok(d) => {
                                                                                t2x.send(ES::DataPlayList(
                                                                                    d,
                                                                                ));
                                                                            }
                                                                            Err(e) => println!(
                                                                                "e {}",
                                                                                e.to_string()
                                                                            ),
                                                                        }
                                                                    }
                                                                    Err(e) => {
                                                                        println!("e {}", e.to_string());
                                                                    }
                                                                }
                                                            }
                                                        });
                                                    }
                                                },
                                            );
                                        }
                                    }
                                }
                                KeyCode::Char('c') => {
                                    if let Some(index) = self.list_state.selected() {
                                        self.ctx.borrow().nc.clear_play_list(
                                            self.list.as_ref().unwrap().list[index].id,
                                        );
                                        self.load_list();
                                    }
                                }
                                KeyCode::Char('k') | KeyCode::Char('j') => {
                                    if ek.code == KeyCode::Char('k') {
                                        self.list_state.select_previous();
                                    } else {
                                        self.list_state.select_next();
                                    }
                                    self.load_list();
                                }
                                KeyCode::Esc => self.focus.back(),
                                KeyCode::Enter => {
                                    self.list_index = self.list_state.selected().unwrap_or(0);
                                    self.focus.set("content");
                                    self.ctx.borrow().tx.send(ES::Render);
                                    return false;
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
            }

            _ => {}
        }
        true
    }

    pub fn render_ref(&mut self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let mut b = Block::new().borders(Borders::RIGHT);
        if self.focus.is_me() {
            b = b.borders(Borders::ALL);
        }
        let inner = b.inner(area);
        b.render(area, buf);
        match &self.list {
            Some(pl) => StatefulWidgetRef::render_ref(
                &(List::new(
                    pl.list
                        .iter()
                        .enumerate()
                        .map(|(i, v)| {
                            let vv = format!(
                                "{}{} {}",
                                if self.list_index == i { "*" } else { "" },
                                if v.play_count > 0 {
                                    &format!("[{}]", self.format_number(v.play_count).as_str())
                                } else {
                                    ""
                                },
                                v.name.as_str()
                            );
                            let mut style = Style::default();
                            if !v.subscribed {
                                style = style.fg(ratatui::style::Color::Blue);
                            }
                            let line = Line::styled(vv, style);

                            ListItem::new(line)
                        })
                        .collect::<Vec<_>>(),
                )
                .highlight_style(Style::new().bg(SLATE.c800).add_modifier(Modifier::BOLD))
                .highlight_symbol(">")),
                inner,
                buf,
                &mut self.list_state,
            ),
            None => "loading".render_ref(inner, buf),
        }
    }
}
