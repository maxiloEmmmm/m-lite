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
    ui::app::Modal,
};

pub struct PlayList {
    cb: Box<dyn Fn(&PlayListItem)>,
    list: Vec<PlayListItem>,
    list_state: ListState,
    list_index: usize,
    title: String,
    close: bool,
}

impl PlayList {
    pub fn new<F>(title: &str, cb: F) -> PlayList
    where
        F: Fn(&PlayListItem) + 'static,
    {
        PlayList {
            cb: Box::new(cb),
            title: title.to_owned(),
            list: vec![],
            list_state: ListState::default(),
            list_index: 0,
            close: false,
        }
    }
}

impl Modal for PlayList {
    fn render_ref(&mut self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let block = Block::bordered().title(Cow::Borrowed(self.title.as_str()));
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
        StatefulWidgetRef::render_ref(
            &(List::new(
                self.list
                    .iter()
                    .enumerate()
                    .map(|(i, v)| {
                        let mut vv = Cow::Borrowed(v.name.as_str());
                        if self.list_index == i {
                            vv = Cow::Owned(format!("*{}", vv));
                        }

                        let line = Line::styled(vv, Style::default());

                        ListItem::new(line)
                    })
                    .collect::<Vec<_>>(),
            )
            .highlight_style(Style::new().bg(SLATE.c800).add_modifier(Modifier::BOLD))
            .highlight_symbol(">")),
            right,
            buf,
            &mut self.list_state,
        );
    }

    fn event(&mut self, e: &mut crate::event::ES) -> bool {
        match e {
            crate::event::ES::DataPlayList(list) => {
                self.list = list
                    .list
                    .iter()
                    .filter(|v| !v.subscribed)
                    .map(|v| v.to_owned())
                    .collect::<Vec<_>>();
                self.list_state.select_first();
                self.list_index = 0;
            }
            crate::event::ES::Event(ee) => match ee {
                Event::Key(k) => match k.code {
                    KeyCode::Char('k') => {
                        self.list_state.select_previous();
                    }
                    KeyCode::Char('j') => {
                        self.list_state.select_next();
                    }
                    KeyCode::Esc => {
                        self.close = true;
                    }
                    KeyCode::Enter => {
                        (self.cb)(&self.list[self.list_state.selected().unwrap_or(0)]);
                        self.close = true;
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
