use std::{borrow::Cow, cell::RefCell, rc::Rc};

use color_eyre::owo_colors::OwoColorize;
use ratatui::{
    crossterm::event::{Event, KeyCode},
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style, palette::tailwind::SLATE},
    text::Line,
    widgets::{Block, Clear, List, ListItem, ListState, StatefulWidgetRef, Widget, WidgetRef},
};

use crate::{
    event::{Play, PlayListOP},
    m163::typ::{PlayItem, PlayListItem},
    play::PlayReq,
    ui::{
        app::{Modal, ShareCtx, global_help},
        widgets::{help::Help, input::Input},
    },
};

pub struct Search {
    list: Vec<PlayItem>,
    list_state: ListState,
    close: bool,
    ctx: ShareCtx,
    list_op: bool,
    input: Input,
}

impl Search {
    pub fn new(ctx: ShareCtx) -> Self {
        Search {
            list: vec![],
            list_state: ListState::default(),
            close: false,
            ctx: ctx.clone(),
            list_op: false,
            input: Input::new(ctx, "条件", |_: String| {}),
        }
    }
}

impl Modal for Search {
    fn render_ref(&mut self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let block = Block::bordered().title(Cow::Borrowed("搜索"));
        let layouts = Layout::new(
            Direction::Horizontal,
            vec![
                Constraint::Percentage(20),
                Constraint::Fill(1),
                Constraint::Percentage(20),
            ],
        )
        .split(area);

        let right = block.inner(layouts[1]);
        Clear.render(layouts[1], buf);
        block.render(layouts[1], buf);
        let ss = Layout::vertical(vec![
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .split(right);
        let block = Block::bordered().title("条件");
        let input_area = block.inner(ss[0]);
        block.render_ref(ss[0], buf);
        self.input.render_inner(input_area, buf);
        format!("共{}个结果", self.list.len()).render_ref(ss[1], buf);
        StatefulWidgetRef::render_ref(
            &(List::new(
                self.list
                    .iter()
                    .enumerate()
                    .map(|(i, v)| {
                        let line = Line::styled(
                            format!(
                                "{} / {}",
                                v.name.as_str(),
                                v.art_r
                                    .first()
                                    .unwrap()
                                    .name
                                    .as_ref()
                                    .unwrap_or(&String::from(""))
                            ),
                            Style::default(),
                        );

                        ListItem::new(line)
                    })
                    .collect::<Vec<_>>(),
            )
            .highlight_style(Style::new().bg(SLATE.c800).add_modifier(Modifier::BOLD))
            .highlight_symbol(">")),
            ss[2],
            buf,
            &mut self.list_state,
        );
    }

    fn event(&mut self, e: &mut crate::event::ES) -> bool {
        if !self.list_op {
            self.input.event_inner(e);
            match e {
                crate::event::ES::Event(ee) => match ee {
                    Event::Key(k) => match k.code {
                        KeyCode::Enter | KeyCode::Esc => {}
                        _ => {
                            return false;
                        }
                    },
                    _ => {}
                },
                _ => {}
            };
        }
        match e {
            crate::event::ES::DataSearch(data) => {
                self.list = data.result.songs.clone();

                self.list_op = true;
                if !self.list.is_empty() {
                    self.list_state.select_first();
                } else {
                    self.list_state.select(None);
                }
            }
            crate::event::ES::Event(ee) => match ee {
                Event::Key(k) => match k.code {
                    KeyCode::Char('h') => {
                        let mut base = vec![
                            (
                                "esc".to_owned(),
                                if !self.list_op {
                                    "返回上一级"
                                } else {
                                    "返回编辑"
                                }
                                .to_owned(),
                            ),
                            (
                                "enter".to_owned(),
                                if self.list_op { "播放" } else { "搜索" }.to_owned(),
                            ),
                            ("q".to_owned(), "退出".to_owned()),
                        ];

                        if self.list_op {
                            base.push(("j/k".to_owned(), "下/上移动".to_owned()));
                        }
                        self.ctx
                            .borrow_mut()
                            .add_modal(Help::new(global_help(base)));
                    }
                    KeyCode::Char('k') => {
                        self.list_state.select_previous();
                    }
                    KeyCode::Char('j') => {
                        self.list_state.select_next();
                    }
                    KeyCode::Esc => {
                        if self.list_op {
                            self.list_op = false;
                        } else {
                            self.close = true;
                        }
                        return false;
                    }
                    KeyCode::Enter => {
                        if !self.list_op {
                            let s = self.input.to_string();
                            if !s.is_empty() {
                                self.ctx.borrow().rt.spawn({
                                    let aux = self.ctx.borrow().async_clone();
                                    async move {
                                        match aux.nc.search(s.as_str()).await {
                                            Ok(d) => {
                                                aux.tx.send(crate::event::ES::DataSearch(d));
                                            }
                                            Err(e) => println!("e {}", e.to_string()),
                                        }
                                    }
                                });
                            }
                        } else {
                            let item = self.list[self.list_state.selected().unwrap_or(0)].clone();
                            let id = item.id;
                            self.ctx
                                .borrow()
                                .tx
                                .send(crate::event::ES::Play(Play::PlayList((
                                    vec![item],
                                    PlayListOP::Append,
                                ))));

                            self.ctx.borrow().ptx.send(PlayReq::Play(id));
                            self.close = true;
                        }
                        return false;
                    }
                    _ => {}
                },
                _ => {}
            },
            _ => {}
        }
        true
    }

    fn closed(&self) -> bool {
        self.close
    }
}
