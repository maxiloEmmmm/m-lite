use std::{
    fmt::{self, Debug},
    ops::Sub,
    sync::Arc,
    time::Duration,
};

use color_eyre::owo_colors::colors::css::Plum;
use ratatui::{
    crossterm::event::{Event, KeyCode},
    layout::{Constraint, Direction, Layout},
    text::Line,
    widgets::{Block, Clear, Paragraph, Widget, WidgetRef},
};

use crate::{event::ES, ui::app::Modal};

#[derive(Clone)]
pub enum Type {
    normal(Duration),
    confirm(bool, Option<Arc<Box<dyn Fn(bool) + Send + Sync>>>),
}

impl Debug for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::normal(d) => f.write_str(&format!("enum normal {}", d.as_secs())),
            Self::confirm(ok, ..) => f.write_str(&format!("enum confirm {}", ok)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Tip {
    pub msg: String,
    pub typ: Type,
    simple: bool,
}

pub fn Confirm(msg: &str, t: Arc<Box<dyn Fn(bool) + Send + Sync>>) -> Tip {
    Tip {
        msg: format!("{} y/n", msg),
        typ: Type::confirm(false, Some(t)),
        simple: false,
    }
}

pub fn Msg(msg: &str, d: Duration) -> Tip {
    Tip {
        msg: msg.to_owned(),
        typ: Type::normal(d),
        simple: false,
    }
}

pub fn SimpleMsg(msg: &str, d: Duration) -> Tip {
    Tip {
        msg: msg.to_owned(),
        typ: Type::normal(d),
        simple: true,
    }
}

impl Modal for Tip {
    fn render_ref(&mut self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let block = Block::bordered().title("tip~");
        let layouts = Layout::new(
            Direction::Horizontal,
            vec![
                Constraint::Percentage(20),
                Constraint::Fill(1),
                Constraint::Percentage(20),
            ],
        )
        .split(area);

        let ss = Layout::vertical(vec![
            Constraint::Fill(1),
            if self.simple {
                Constraint::Length(3)
            } else {
                Constraint::Percentage(40)
            },
            Constraint::Fill(1),
        ])
        .split(layouts[1]);
        let mid = block.inner(ss[1]);
        Clear.render(ss[1], buf);
        block.render(ss[1], buf);
        Paragraph::new(self.msg.as_str())
            .wrap(ratatui::widgets::Wrap { trim: true })
            .render_ref(mid, buf);
    }

    fn event(&mut self, e: &mut ES) -> bool {
        match &mut self.typ {
            Type::confirm(done, ok_cb) => match e {
                ES::Event(ee) => match ee {
                    Event::Key(k) => match k.code {
                        KeyCode::Char('y') | KeyCode::Char('n') => {
                            *done = true;
                            if let Some(cb) = ok_cb {
                                cb(KeyCode::Char('y').eq(&k.code));
                            }
                            return false;
                        }
                        _ => {}
                    },
                    _ => {}
                },
                _ => {}
            },
            Type::normal(d) => match e {
                ES::SEC => {
                    *d = d.saturating_sub(Duration::from_secs(1));
                }
                _ => {}
            },
            _ => {}
        }
        true
    }

    fn closed(&self) -> bool {
        match self.typ {
            Type::confirm(done, ..) => done.clone(),
            Type::normal(dur) => dur.as_millis() == 0,
        }
    }
}
