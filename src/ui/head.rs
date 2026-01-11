use ratatui::{
    crossterm::event::{Event, KeyCode},
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Widget, WidgetRef},
};

use crate::{
    event::{ES, HeadMenuKey},
    ui::{
        app::{ShareCtx, Wrap, global_help},
        focus::Focus,
        widgets::help::Help,
    },
};

pub struct Head {
    name: String,
    desc: String,
    focus: Focus,
    index: usize,
    pos: usize,
    list: Vec<HEAD_MENU>,
    ctx: ShareCtx,
}

pub struct HEAD_MENU {
    Title: String,
    Key: HeadMenuKey,
}

impl WidgetRef for Head {
    fn render_ref(&self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let mut b = Block::default();
        if self.focus.is_me() {
            b = b.borders(Borders::ALL);
        }
        let inner = b.inner(area);

        let p = Paragraph::new(format!(
            "{} - {}{}{}",
            self.ctx.borrow().maybe_hidden(self.name.as_str()),
            self.ctx.borrow().maybe_hidden(self.desc.as_str()),
            if self.ctx.borrow().private {
                "[private mode]"
            } else {
                ""
            },
            if self.ctx.borrow().offline {
                "(offline...)"
            } else {
                ""
            }
        ));
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1), Constraint::Length(1)])
            .split(inner);
        b.render(area, buf);
        p.render_ref(layout[0], buf);
        let mods = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Length(4),
                Constraint::Fill(1),
                Constraint::Fill(1),
                Constraint::Fill(1),
                Constraint::Fill(1),
                Constraint::Length(4),
            ])
            .split(layout[1]);
        "<-".render(mods[0], buf);
        self.list.iter().enumerate().for_each(|(index, v)| {
            Paragraph::new(
                (if self.pos == index { "* " } else { "" }).to_owned() + v.Title.as_str(),
            )
            .centered()
            .style(Style::default().fg(if self.index == index {
                ratatui::style::Color::Green
            } else {
                Color::Reset
            }))
            .render(mods[index + 1], buf);
        });
        "->".render(mods[5], buf);
    }
}

impl Head {
    pub fn new(ctx: ShareCtx, focus: Focus) -> Self {
        Head {
            name: String::from("loading..."),
            desc: String::new(),
            focus: focus,
            index: 0,
            pos: 0,
            list: vec![
                HEAD_MENU {
                    Title: "我的音乐".to_owned(),
                    Key: HeadMenuKey::My,
                },
                HEAD_MENU {
                    Title: "推荐".to_owned(),
                    Key: HeadMenuKey::Maybe,
                },
                HEAD_MENU {
                    Title: "关于".to_owned(),
                    Key: HeadMenuKey::About,
                },
            ],
            ctx: ctx,
        }
    }

    pub fn change_module(&mut self, key: HeadMenuKey) {
        self.focus.set("slide");
        self.ctx.borrow().tx.send(ES::RuntimeHead(key.clone()));

        match key {
            HeadMenuKey::My => {
                self.ctx.borrow().rt.spawn({
                    let ctx = self.ctx.borrow().async_clone();
                    async move {
                        match ctx.nc.play_list(0, 10).await {
                            Ok(vv) => {
                                ctx.tx.send(ES::DataPlayList(vv));
                            }
                            Err(err) => ctx.tx.wrap_error("req top.play_list", &err),
                        }
                    }
                });
            }
            HeadMenuKey::Maybe => {
                self.ctx.borrow().rt.spawn({
                    let ctx = self.ctx.borrow().async_clone();
                    async move {
                        match ctx.nc.recommend_resource().await {
                            Ok(vv) => {
                                ctx.tx.send(ES::DataRecommendResource(vv));
                            }
                            Err(err) => ctx.tx.wrap_error("req recommed.resource", &err),
                        }
                    }
                });
            }
            _ => {}
        }
    }

    pub fn event(&mut self, e: &mut ES) -> bool {
        match e {
            ES::DataProfile(profile) => {
                self.ctx.borrow_mut().offline = false;
                self.name = profile.profile.nickname.to_owned();
                self.desc = profile.profile.signature.as_ref().map(|v| v.to_owned()).unwrap_or("".to_owned());
            }
            ES::Event(ee) => {
                if self.focus.is_me() {
                    match ee {
                        Event::Key(ek) => match ek.code {
                            KeyCode::Char('h') => {
                                self.ctx.borrow_mut().add_modal(Help::new(global_help(vec![
                                    ("j/k".to_owned(), "右/左移动".to_owned()),
                                    ("enter".to_owned(), "进入模块".to_owned()),
                                    ("p".to_owned(), "打开播放列表".to_owned()),
                                ])));
                            }
                            KeyCode::Char('j') => {
                                self.pos += 1;
                                if self.pos > 1 {
                                    self.pos = 0;
                                }
                            }
                            KeyCode::Char('k') => {
                                if self.pos != 0 {
                                    self.pos -= 1;
                                }
                            }
                            KeyCode::Enter => {
                                self.index = self.pos;
                                self.change_module(self.list[self.index].Key.clone());
                                return false;
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        true
    }
}
