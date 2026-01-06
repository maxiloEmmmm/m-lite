use ratatui::{
    crossterm::event::{Event, KeyCode},
    layout::{Constraint, Direction, Layout},
    style::Style,
    text::Line,
    widgets::{Clear, List, ListItem, Widget, WidgetRef},
};

use crate::ui::{
    app::{Modal, ShareCtx, global_help},
    footer::{Footer, Lyric},
    widgets::help::Help,
};

const VIEW_PRE_LINES: usize = 20;

pub struct Zero {
    list: Vec<Lyric>,
    close: bool,
    offset: u16,
    current: usize,
    max: u16,
    ctx: ShareCtx,
}

impl Zero {
    pub fn new(ctx: ShareCtx) -> Self {
        Zero {
            list: vec![],
            close: false,
            current: 0,
            offset: 0,
            max: 0,
            ctx,
        }
    }
}

impl Modal for Zero {
    fn render_ref(&mut self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let mut high = VIEW_PRE_LINES * 2 + 1;
        let mut half = VIEW_PRE_LINES;
        if high > area.height as usize {
            high = area.height as usize;
            half = high / 2;
        }
        let layouts = Layout::vertical(vec![
            Constraint::Fill(1),
            Constraint::Min(high as u16),
            Constraint::Fill(1),
        ])
        .split(
            Layout::new(
                Direction::Horizontal,
                vec![
                    Constraint::Fill(1),
                    Constraint::Min(if self.max > area.width {
                        area.width
                    } else {
                        self.max
                    }),
                    Constraint::Fill(1),
                ],
            )
            .split(area)[1],
        );

        Clear.render(area, buf);
        if self.list.is_empty() {
            "什么也没有~".render(layouts[1], buf);
            return;
        }
        let start = self.current.saturating_sub(half);
        let mut end = self.current + half;
        if self.current - start < half {
            // 前面不够后面就多补点
            end += half - self.current;
        }
        if end >= self.list.len() {
            end = self.list.len() - 1;
        }
        List::new(
            self.list[start..=end]
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    let mut style = Style::default();
                    if self.current == start + i {
                        style = style.fg(ratatui::style::Color::Green);
                    }

                    let line = Line::styled(
                        if v.text.is_empty() {
                            "........"
                        } else {
                            v.text.as_str()
                        },
                        style,
                    )
                    .centered();

                    ListItem::new(line)
                })
                .collect::<Vec<_>>(),
        )
        .render_ref(layouts[1], buf);
    }

    fn event(&mut self, e: &mut crate::event::ES) -> bool {
        match e {
            crate::event::ES::Play(p) => match p {
                crate::event::Play::Offset(d) => {
                    self.offset = d.as_secs() as u16;
                    if !self.list.is_empty() {
                        self.current = Footer::get_lyric(self.offset, &self.list).unwrap_or(0);
                    }
                }
                crate::event::Play::Lyric(d) => {
                    self.list = d.clone();

                    let mut max = 0_u16;
                    d.iter().for_each(|v| {
                        if v.text.len() as u16 > max {
                            max = v.text.len() as u16;
                        }
                    });
                    self.max = max;
                    self.current = Footer::get_lyric(self.offset, &self.list).unwrap_or(0);
                }
                _ => {}
            },
            crate::event::ES::Event(ee) => match ee {
                Event::Key(k) => match k.code {
                    KeyCode::Esc => {
                        self.close = true;
                    }
                    KeyCode::Char('z') => {
                        self.close = true;
                        return false;
                    }
                    KeyCode::Char('h') => {
                        let mut base = global_help(vec![("esc".to_owned(), "退出纯净".to_owned())]);

                        self.ctx.borrow_mut().add_modal(Help::new(base));
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
