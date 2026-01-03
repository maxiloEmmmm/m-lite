use rand::Fill;
use ratatui::{
    crossterm::event::{Event, KeyCode},
    layout::{Constraint, Layout},
    widgets::{Block, Clear, Paragraph, Widget, WidgetRef},
};

use crate::ui::app::Modal;

pub struct Help {
    hs: Vec<(String, String)>,
    close: bool,
}

impl Help {
    pub fn new(hs: Vec<(String, String)>) -> Self {
        Help { hs, close: false }
    }
}

impl Modal for Help {
    fn render_ref(&mut self, mut area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let mut layouts = Layout::horizontal([
            Constraint::Percentage(20),
            Constraint::Fill(1),
            Constraint::Percentage(20),
        ])
        .split(area);
        layouts = Layout::vertical(vec![
            Constraint::Percentage(30),
            Constraint::Fill(1),
            Constraint::Percentage(30),
        ])
        .split(layouts[1]);

        let col_constraints = (0..4).map(|_| Constraint::Fill(1));
        let row_constraints = (0..self.hs.len().div_ceil(4)).map(|_| Constraint::Length(1));
        let horizontal = Layout::horizontal(col_constraints).spacing(1);
        let vertical = Layout::vertical(row_constraints);

        let block = Block::bordered().title("help");

        Clear.render_ref(layouts[1], buf);
        area = block.inner(layouts[1]);
        block.render_ref(layouts[1], buf);
        let rows = vertical.split(area);
        let cells = rows.iter().flat_map(|&row| horizontal.split(row).to_vec());
        for (i, cell) in cells.enumerate() {
            if i < self.hs.len() {
                Paragraph::new(format!(
                    "[{}] {}",
                    self.hs[i].0.as_str(),
                    self.hs[i].1.as_str()
                ))
                .render(cell, buf);
            }
        }
    }

    fn event(&mut self, e: &mut crate::event::ES) -> bool {
        match e {
            crate::event::ES::Event(e) => match e {
                Event::Key(k) => {
                    match k.code {
                        KeyCode::Esc => {
                            self.close = true;
                        }
                        _ => {}
                    }

                    return false;
                }
                _ => {}
            },
            _ => {}
        };
        true
    }

    fn closed(&self) -> bool {
        self.close
    }
}
