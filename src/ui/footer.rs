use std::{
    borrow::Cow,
    collections::HashSet,
    ops::{Add, Index, Sub},
    time::Duration,
};

use color_eyre::owo_colors::OwoColorize;
use ratatui::{
    crossterm::event::{Event, KeyCode, ModifierKeyCode},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style, palette::tailwind::SLATE},
    text::Line,
    widgets::{Block, Clear, List, ListItem, ListState, StatefulWidgetRef, Widget, WidgetRef},
};

use crate::{
    event::{ES, Play, PlayListOP, PlayMode, PlayState},
    m163::typ::PlayItem,
    play::PlayReq,
    ui::{
        app::{ShareCtx, global_help},
        focus::Focus,
        widgets::{help::Help, play_list::PlayList as PlayListWidget, tip::Msg},
        zero::Zero,
    },
};

#[derive(Clone, Debug)]
pub struct Lyric {
    pub text: String,
    pub duration: u16,
}

pub struct Footer {
    state: PlayState,
    current: Option<PlayItem>,
    offset: Duration,
    ctx: ShareCtx,
    list: Vec<PlayItem>,
    list_state: ListState,
    list_index: usize,
    play_mode: PlayMode,
    list_view: bool,
    plFocus: Focus, // todo 把播放列表迁移到 modal
    volume: f32,
    lyrics: Vec<Lyric>,
    bad: HashSet<usize>,
}

impl Footer {
    pub fn new(ctx: ShareCtx, play_list_focus: Focus) -> Self {
        let volume = ctx.borrow().config.volume;
        Footer {
            state: PlayState::None,
            offset: Duration::from_secs(0),
            play_mode: PlayMode::Single,
            list_view: false,
            list: vec![],
            current: None,
            list_state: ListState::default(),
            list_index: 0,
            plFocus: play_list_focus,
            volume: volume,
            lyrics: vec![],
            bad: HashSet::new(),
            ctx: ctx,
        }
    }
    fn set_play_mode(&mut self, mode: PlayMode) {
        self.play_mode = mode;
        match self.play_mode {
            PlayMode::Random => {
                fastrand::shuffle(&mut self.list);
            }
            _ => {}
        }
    }

    fn play_next(&mut self, first: bool) {
        if self.list.is_empty() {
            return;
        }

        let mut index: usize;
        match self.play_mode {
            PlayMode::SingleLoop => {
                index = self.list_state.selected().unwrap_or(0);
            }
            _ => {
                if first {
                    index = 0;
                } else {
                    index = self.list_state.selected().map(|v| v + 1).unwrap_or(0);
                    if index == self.list.len() {
                        if matches!(self.play_mode, PlayMode::Random) {
                            fastrand::shuffle(&mut self.list);
                        }

                        index = 0
                    }
                }
            }
        }

        let id = self.list[index].id;
        if self.bad.contains(&id) {
            if !matches!(self.play_mode, PlayMode::SingleLoop) || self.list.len() == 1 {
                return;
            }
            self.play_next(false);
            return;
        }
        self.ctx.borrow().ptx.send(PlayReq::Play(id));
    }

    fn change_t(&mut self, mut v: isize) {
        if self.current.is_none() {
            return;
        }
        if v >= 0 {
            let vv = self.current.as_ref().unwrap().dt / 1000 - self.offset.as_secs();
            if vv < v as u64 {
                v = vv as isize;
            }
            self.offset = self.offset.add(Duration::from_secs(v as u64));
        } else {
            if self.offset.as_secs() < -v as u64 {
                v = -(self.offset.as_secs() as isize);
            }
            self.offset = self.offset.sub(Duration::from_secs(-v as u64));
        }
        self.ctx.borrow().ptx.send(PlayReq::T(self.offset.clone()));
    }
    pub fn event(&mut self, e: &mut ES) {
        match e {
            ES::Play(p) => match p {
                Play::State(s) => {
                    self.state = s.clone();
                    match s {
                        PlayState::Failed(id) => {
                            self.bad.insert(*id);
                            self.play_next(false);
                        }
                        PlayState::Play(id, lyric) => {
                            self.bad.remove(id);
                            self.lyrics.clear();
                            for line in lyric.split('\n') {
                                if let Some(index) = line.find('[') {
                                    if index == 0 {
                                        if let Some(rIndex) = line.find(']') {
                                            let dts = &line[index + 1..rIndex];
                                            let mut dt = 0_u16;
                                            for (num, v) in dts.split(':').enumerate() {
                                                match num {
                                                    0 => {
                                                        dt += v.parse().unwrap_or(0) * 60;
                                                    }
                                                    1 => {
                                                        dt += v[..v.find('.').unwrap_or(v.len())]
                                                            .parse()
                                                            .unwrap_or(0);
                                                    }
                                                    _ => {}
                                                }
                                            }
                                            self.lyrics.push(Lyric {
                                                text: line[rIndex + 1..].to_owned(),
                                                duration: dt,
                                            });
                                        }
                                    }
                                }
                            }
                            self.ctx
                                .borrow()
                                .tx
                                .send(ES::Play(Play::Lyric(self.lyrics.clone())));
                            for (index, v) in self.list.iter().enumerate() {
                                if v.id.eq(id) {
                                    self.current = Some(v.clone());
                                    self.list_state.select(Some(index));
                                    self.list_index = index;
                                    break;
                                }
                            }
                            self.offset = Duration::from_secs(0);
                        }
                        _ => {}
                    }
                }
                Play::Offset(o) => {
                    self.offset = o.clone();
                }
                Play::PlayMode(s) => {
                    self.set_play_mode(s.clone());
                }
                Play::PlayList((list, op)) => {
                    match op {
                        PlayListOP::Set => {
                            self.bad.clear();
                            self.list = list.clone();
                        }
                        PlayListOP::Append => {
                            for vv in list {
                                if self.list.iter().find(|v| v.id.eq(&vv.id)).is_none() {
                                    self.list.push(vv.clone());
                                }
                            }
                        }
                    }

                    if matches!(self.play_mode, PlayMode::Random) {
                        fastrand::shuffle(&mut self.list);
                    }
                }
                _ => {}
            },
            ES::SEC => match self.state {
                PlayState::Start => {
                    let offset = Duration::from_secs(self.current.as_ref().unwrap().dt / 1000);
                    if self.offset.lt(&offset) {
                        self.offset = self.offset.add(Duration::from_secs(1));
                        self.ctx
                            .borrow()
                            .tx
                            .send(ES::Play(Play::Offset(self.offset.clone())));
                    }

                    let end = self.offset.eq(&offset);
                    if end {
                        self.play_next(false);
                    }

                    if end || !self.ctx.borrow().config.less_usage {
                        self.ctx.borrow().tx.send(ES::Render);
                    }
                }
                _ => {}
            },
            ES::Event(e) => match e {
                Event::Key(ek) => match ek.code {
                    KeyCode::Char('h') => {
                        if self.plFocus.is_me() {
                            let mut base = global_help(vec![
                                ("t".to_owned(), "加入歌单".to_owned()),
                                ("j/k".to_owned(), "下/上移动".to_owned()),
                                ("enter".to_owned(), "播放选中歌曲".to_owned()),
                                ("r".to_owned(), "随机播放列表".to_owned()),
                                ("o".to_owned(), "顺序播放列表".to_owned()),
                                ("s".to_owned(), "单曲循环".to_owned()),
                                ("p".to_owned(), "打开播放列表".to_owned()),
                            ]);

                            self.ctx.borrow_mut().add_modal(Help::new(base));
                        }
                    }
                    KeyCode::Char('t') => {
                        if self.plFocus.is_me() {
                            if self.list_index >= self.list.len() {
                                return;
                            }
                            let id = self.list[self.list_index].id;
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
                    KeyCode::Char('p') => {
                        self.list_view = !self.list_view;
                        if self.list_view {
                            self.plFocus.set("play_list");
                        } else {
                            self.plFocus.back();
                        }
                    }
                    KeyCode::Char('z') => {
                        self.ctx
                            .borrow_mut()
                            .add_full_modal(Zero::new(self.ctx.clone()));
                        self.ctx
                            .borrow()
                            .tx
                            .send(ES::Play(Play::Lyric(self.lyrics.clone())));
                    }
                    KeyCode::Char(' ') => match self.state {
                        PlayState::Start => {
                            self.ctx.borrow().ptx.send(PlayReq::Stop);
                        }
                        PlayState::Stop => {
                            self.ctx.borrow().ptx.send(PlayReq::Start);
                        }
                        _ => {}
                    },
                    KeyCode::Char('>') => {
                        self.change_t(10);
                    }
                    KeyCode::Char('<') => {
                        self.change_t(-10);
                    }
                    KeyCode::Char('k') => {
                        if self.plFocus.is_me() {
                            self.list_state.select_previous();
                        }
                    }
                    KeyCode::Char('j') => {
                        if self.plFocus.is_me() {
                            self.list_state.select_next();
                        }
                    }
                    KeyCode::Char('r') => {
                        if self.plFocus.is_me() {
                            self.set_play_mode(PlayMode::Random);
                            self.play_next(true);
                        }
                    }
                    KeyCode::Char('o') => {
                        if self.plFocus.is_me() {
                            self.set_play_mode(PlayMode::Order);
                        }
                    }
                    KeyCode::Char('s') => {
                        if self.plFocus.is_me() {
                            self.set_play_mode(PlayMode::SingleLoop);
                        }
                    }
                    KeyCode::Char('+') => {
                        self.volume += 0.1;
                        if self.volume > 1.0 {
                            self.volume = 1.0;
                        }
                        self.ctx.borrow().ptx.send(PlayReq::V(self.volume));
                    }
                    KeyCode::Char('-') => {
                        self.volume -= 0.1;
                        if self.volume < 0.0 {
                            self.volume = 0.0;
                        }
                        self.ctx.borrow().ptx.send(PlayReq::V(self.volume));
                    }
                    KeyCode::Esc => {
                        if self.plFocus.is_me() {
                            self.plFocus.back();
                            self.list_view = false;
                        }
                    }
                    KeyCode::Enter => {
                        if self.plFocus.is_me() {
                            if !self.list.is_empty() {
                                let id = self.list[self.list_state.selected().unwrap_or(0)].id;
                                self.ctx.borrow().ptx.send(PlayReq::Play(id));
                            }
                        }
                    }
                    _ => {}
                },
                _ => {}
            },
            _ => {}
        }
    }

    pub fn get_lyric(offset: u16, lyrics: &[Lyric]) -> Option<usize> {
        let len = lyrics.len();
        if len == 0 {
            return None;
        }
        for index in 0..len {
            if index == len - 1 {
                break;
            }
            if offset >= lyrics[index].duration && offset < lyrics[index + 1].duration {
                return Some(index);
            }
        }

        Some(len - 1)
    }

    pub fn render_ref(
        &mut self,
        top: Rect,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
    ) {
        match self.state {
            PlayState::None => {
                "-".render_ref(area, buf);
            }
            PlayState::Start | PlayState::Stop => {
                format!(
                    "{} {}/{} {} {} 音量 {}% {}",
                    if matches!(self.state, PlayState::Start) {
                        "playing..."
                    } else {
                        "stopped!"
                    },
                    self.current.as_ref().unwrap().name,
                    self.current
                        .as_ref()
                        .unwrap()
                        .art_r
                        .first()
                        .unwrap()
                        .name
                        .as_ref()
                        .unwrap_or(&String::from("")),
                    if self.ctx.borrow().config.less_usage {
                        "less-cpu"
                    } else {
                        &format!(
                            "{:02}:{:02}/{:02}:{:02}",
                            self.offset.as_secs() / 60,
                            self.offset.as_secs() % 60,
                            self.current.as_ref().unwrap().dt / 1000 / 60,
                            self.current.as_ref().unwrap().dt / 1000 % 60
                        )
                    },
                    match self.play_mode {
                        PlayMode::SingleLoop => "单曲循环",
                        PlayMode::Random => "随机",
                        PlayMode::Single => "单曲",
                        _ => "列表循环",
                    },
                    (self.volume * 100 as f32).ceil(),
                    if self.ctx.borrow().config.less_usage {
                        ""
                    } else {
                        Self::get_lyric(self.offset.as_secs() as u16, self.lyrics.as_slice())
                            .map(|v| self.lyrics[v].text.as_str())
                            .unwrap_or("")
                    },
                )
                .render(area, buf);
            }
            _ => {
                "stop...".render(area, buf);
            }
        }

        if self.list_view {
            let block = Block::bordered().title("播放列表");
            let layouts = Layout::new(
                Direction::Horizontal,
                vec![Constraint::Percentage(50), Constraint::Fill(1)],
            )
            .split(top);

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

                            if self.ctx.borrow().like_set.get(&v.id).is_some() {
                                vv = Cow::Owned(format!("{} 💗", vv));
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
    }
}
