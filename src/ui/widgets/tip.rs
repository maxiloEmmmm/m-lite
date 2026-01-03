use std::{
    fmt::{self, Debug},
    ops::Sub,
    sync::Arc,
    time::Duration,
};

use color_eyre::owo_colors::colors::css::Plum;
use ratatui::{
    crossterm::event::{Event, KeyCode},
    text::Line,
    widgets::{Block, Clear, WidgetRef},
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
}

pub fn Confirm(msg: &str, t: Arc<Box<dyn Fn(bool) + Send + Sync>>) -> Tip {
    Tip {
        msg: format!("{} y/n", msg),
        typ: Type::confirm(false, Some(t)),
    }
}

pub fn Msg(msg: &str, d: Duration) -> Tip {
    Tip {
        msg: msg.to_owned(),
        typ: Type::normal(d),
    }
}

impl Modal for Tip {
    fn render_ref(&mut self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let block = Block::bordered().title("tip~");
        let line = Line::from(self.msg.as_str());
        let top = ratatui::layout::Rect::new(
            0,
            0,
            if line.width() as u16 + 2 > area.width {
                area.width
            } else {
                line.width() as u16 + 2
            },
            3,
        );
        let area = block.inner(top);
        Clear.render_ref(top, buf);
        block.render_ref(top, buf);
        line.render_ref(area, buf);
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
