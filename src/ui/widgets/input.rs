use std::borrow::Cow;

use color_eyre::owo_colors::OwoColorize;
use ratatui::{
    crossterm::event::{Event, KeyCode},
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style, palette::tailwind::SLATE},
    text::Line,
    widgets::{Block, Clear, List, ListItem, ListState, StatefulWidgetRef, Widget, WidgetRef},
};

use crate::{
    m163::typ::{PlayItem, PlayListItem},
    ui::app::{Modal, ShareCtx},
};

pub struct Input {
    cb: Box<dyn Fn(String)>,
    v: Vec<char>,
    title: String,
    close: bool,
    cursor: bool,
    ctx: ShareCtx,
}

impl Input {
    pub fn new<F>(ctx: ShareCtx, title: &str, cb: F) -> Input
    where
        F: Fn(String) + 'static,
    {
        Input {
            cb: Box::new(cb),
            title: title.to_owned(),
            v: vec![' '],
            close: false,
            cursor: false,
            ctx,
        }
    }

    pub fn to_string(&self) -> String {
        String::from_iter(&self.v[..self.v.len() - 1])
    }
}

impl Input {
    pub fn render_inner(
        &mut self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
    ) {
        let len = self.v.len();
        self.v[len - 1] = if self.cursor { '|' } else { ' ' };
        self.v.iter().collect::<String>().render_ref(area, buf);
    }
    pub fn event_inner(&mut self, e: &mut crate::event::ES) -> bool {
        match e {
            crate::event::ES::SEC => {
                self.cursor = !self.cursor;
                self.ctx.borrow().tx.send(crate::event::ES::Render);
            }
            crate::event::ES::Event(ee) => match ee {
                Event::Key(k) => {
                    match k.code {
                        KeyCode::Esc => {
                            self.close = true;
                        }
                        KeyCode::Backspace => {
                            if self.v.len() > 1 {
                                self.v.pop();
                                let len = self.v.len();
                                self.v[len - 1] = if self.cursor { '|' } else { ' ' };
                            }
                        }
                        KeyCode::Char(v) => {
                            let len = self.v.len();
                            self.v[len - 1] = v;
                            self.v.push(if self.cursor { '|' } else { ' ' });
                        }
                        KeyCode::Enter => {
                            (self.cb)(self.to_string());
                            self.close = true;
                        }
                        _ => {}
                    }
                    return false;
                }
                _ => {}
            },
            _ => {}
        }
        true
    }
}

impl Modal for Input {
    fn render_ref(&mut self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let block = Block::bordered().title(Cow::Borrowed(self.title.as_str()));
        let mut layouts = Layout::new(
            Direction::Horizontal,
            vec![
                Constraint::Percentage(20),
                Constraint::Fill(1),
                Constraint::Percentage(20),
            ],
        )
        .split(area);
        layouts = Layout::vertical(vec![Constraint::Percentage(20), Constraint::Length(3)])
            .split(layouts[1]);

        let right = block.inner(layouts[1]);
        Clear.render(layouts[1], buf);
        block.render_ref(layouts[1], buf);
        self.render_inner(right, buf);
    }

    fn event(&mut self, e: &mut crate::event::ES) -> bool {
        self.event_inner(e)
    }

    fn closed(&self) -> bool {
        self.close
    }
}
