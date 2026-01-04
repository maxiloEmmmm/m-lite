use core::fmt;
use std::{
    cell::{RefCell, RefMut},
    collections::{HashSet, LinkedList},
    fmt::Display,
    rc::Rc,
    sync::{
        Arc,
        mpsc::{self, Sender},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use qrcode::QrCode;
use ratatui::{
    Frame,
    buffer::Buffer,
    crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers},
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Clear, Widget, WidgetRef},
};
use tokio::sync::mpsc::UnboundedSender;
use tui_qrcode::QrCodeWidget;

use crate::{
    config::Config,
    event::{AppState, ES, HeadMenuKey, LoginState},
    m163::{
        self,
        client::{Nc, TARGET},
    },
    play::PlayReq,
    ui::{
        content::Content,
        focus::Focus,
        footer::Footer,
        head::Head,
        search::Search,
        slide::Slide,
        widgets::tip::{Confirm, Msg},
    },
};

pub struct Context {
    pub nc: Arc<m163::client::Nc>,
    pub tx: mpsc::Sender<ES>,
    pub rt: Arc<tokio::runtime::Runtime>,
    pub ptx: UnboundedSender<PlayReq>,
    pub config: Config,
    pub test: i64,
    pub like_set: HashSet<usize>,
    pub like_play_id: usize,
    pub modals: Vec<Rc<RefCell<Box<dyn Modal>>>>,
    pub offline: bool,
}

#[derive(Clone)]
pub struct AsyncUtil {
    pub nc: Arc<m163::client::Nc>,
    pub tx: mpsc::Sender<ES>,
    pub rt: Arc<tokio::runtime::Runtime>,
    pub ptx: UnboundedSender<PlayReq>,
}

pub trait Wrap {
    fn wrap_error(&self, what: &str, err: &impl ToString);
}

impl Wrap for Sender<ES> {
    fn wrap_error(&self, what: &str, err: &impl ToString) {
        self.send(ES::Tip(Msg(
            &format!("[error]{}: {}", what, err.to_string()),
            Duration::from_secs(3),
        )));
    }
}

impl Context {
    pub fn info(&mut self, msg: &str) {
        self.add_modal(Msg(msg, Duration::from_millis(1500)));
    }

    pub fn confirm<F>(&mut self, msg: &str, cb: F)
    where
        F: Fn(bool) + Send + Sync + 'static,
    {
        self.add_modal(Confirm(msg, Arc::new(Box::new(cb))));
    }

    pub fn add_modal<T: Modal + 'static>(&mut self, m: T) {
        self.modals.push(Rc::new(RefCell::new(Box::new(m))));
    }

    pub fn async_clone(&self) -> AsyncUtil {
        AsyncUtil {
            nc: self.nc.clone(),
            tx: self.tx.clone(),
            rt: self.rt.clone(),
            ptx: self.ptx.clone(),
        }
    }
}

pub type ShareCtx = Rc<RefCell<Context>>;

pub fn global_help(mut base: Vec<(String, String)>) -> Vec<(String, String)> {
    base.append(&mut vec![
        ("space".to_owned(), "开始/暂停".to_owned()),
        ("p".to_owned(), "打开播放列表".to_owned()),
        ("</>".to_owned(), "快退/快进".to_owned()),
        ("-/+".to_owned(), "调整音量".to_owned()),
        ("q".to_owned(), "退出".to_owned()),
        ("f".to_owned(), "搜索".to_owned()),
        ("z".to_owned(), "纯净模式".to_owned()),
    ]);
    base
}

pub trait Modal {
    fn event(&mut self, e: &mut ES) -> bool;
    fn render_ref(&mut self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer);
    fn closed(&self) -> bool;
}

pub struct App {
    pub ctx: ShareCtx,
    head: Head,
    slide: Slide,
    content: Content,
    footer: Footer,
    pub state: AppState,
    login_qr: Option<QrCode>,
    login_chain: String,
    login_key: String,
    login_state: LoginState,
    last_offline_test: Instant,
}

impl App {
    pub fn new(
        nc: Arc<Nc>,
        event_tx: Sender<ES>,
        play_tx: UnboundedSender<PlayReq>,
        config: Config,
    ) -> Self {
        let focus = Focus::root();

        let slideFocus = focus.sub("slide");
        let contentFocus = slideFocus.sub("content");
        let playListFocus = contentFocus.sub("play_list");
        let rt = tokio::runtime::Runtime::new().unwrap();
        let ctx = Rc::new(RefCell::new(Context {
            tx: event_tx,
            nc: nc,
            rt: Arc::new(rt),
            ptx: play_tx.clone(),
            config,
            test: 0,
            like_set: HashSet::new(),
            like_play_id: 0,
            modals: vec![],
            offline: false,
        }));

        let top = ctx.clone();
        App {
            ctx: ctx.clone(),
            state: if top.borrow().nc.is_login() {
                AppState::Authed
            } else {
                AppState::Authing
            },
            login_key: "".to_owned(),
            login_chain: "".to_owned(),
            login_qr: None,
            login_state: LoginState::Wait,
            head: Head::new(ctx.clone(), focus),
            slide: Slide::new(ctx.clone(), slideFocus),
            content: Content::new(ctx.clone(), contentFocus),
            footer: Footer::new(ctx.clone(), playListFocus),
            last_offline_test: Instant::now(),
        }
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        if matches!(self.state, AppState::Authing) {
            let layouts = Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![Constraint::Length(1), Constraint::Fill(1)])
                .split(frame.area());
            let mut state = "请扫码";
            match self.login_state {
                LoginState::Authing => {
                    state = "授权中";
                }
                LoginState::Failed => {
                    state = "失败!";
                }
                _ => {}
            }
            format!("认证, {}...", state,).render(layouts[0], frame.buffer_mut());
            if let Some(qr) = self.login_qr.clone() {
                let widget = QrCodeWidget::new(qr).colors(tui_qrcode::Colors::Inverted);
                frame.render_widget(widget, layouts[1]);
            }
        } else {
            let layouts = Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![
                    Constraint::Length(4),
                    Constraint::Fill(9),
                    Constraint::Length(1),
                ])
                .split(frame.area());
            self.head.render_ref(layouts[0], frame.buffer_mut());

            let main = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(vec![Constraint::Fill(3), Constraint::Fill(7)])
                .split(layouts[1]);

            self.slide.render_ref(main[0], frame.buffer_mut());
            self.content.render_ref(main[1], frame.buffer_mut());
            self.footer
                .render_ref(frame.area(), layouts[2], frame.buffer_mut());
        }

        {
            let len = self.ctx.borrow_mut().modals.len();
            for m in 0..len {
                let modal = self.ctx.borrow().modals[m].clone();
                modal
                    .borrow_mut()
                    .render_ref(frame.area(), frame.buffer_mut());
            }
        }
    }

    pub fn event(&mut self, e: &mut ES) {
        {
            let m = self.ctx.borrow().modals.len();
            let mut ret = false;
            for m in (0..m).rev() {
                // todo iter_mut? can't as_mut ? why
                let modal = self.ctx.borrow().modals[m].clone();
                ret = ret || !modal.borrow_mut().event(e);
                if ret {
                    break; // 别立马退出 可能 modal 已经要移除视图了 先检查
                }
            }
            self.ctx
                .borrow_mut()
                .modals
                .retain_mut(|v| !v.borrow().closed());
            if !m.eq(&self.ctx.borrow().modals.len()) {
                self.ctx.borrow().tx.send(ES::Render);
            }

            if ret {
                return;
            }
        }
        match &e {
            ES::Event(e) => match e {
                Event::Key(ee) => match ee.code {
                    KeyCode::Char('f') => {
                        self.ctx
                            .borrow_mut()
                            .add_modal(Search::new(self.ctx.clone()));
                    }
                    _ => {}
                },
                _ => {}
            },
            ES::Tip(tip) => {
                self.ctx.borrow_mut().add_modal(tip.clone());
            }
            ES::LoginState(s) => {
                self.login_state = s.clone();
            }
            ES::AppState(s) => {
                self.state = s.clone();
                match s {
                    AppState::Authed => {
                        self.ctx.borrow().rt.spawn({
                            let txx = self.ctx.borrow().tx.clone();
                            let nnx = self.ctx.borrow().nc.clone();
                            async move {
                                match nnx.profile().await {
                                    Ok(v) => {
                                        txx.send(ES::Event(Event::Key(KeyEvent::new(
                                            ratatui::crossterm::event::KeyCode::Enter,
                                            KeyModifiers::empty(),
                                        )))); // todo remove replace by head method
                                        txx.send(ES::DataProfile(v));
                                    }
                                    Err(err) => {
                                        match err {
                                            m163::client::NCErr::Offline => {}
                                            _ => {
                                                // 获取不到就重新认证
                                                txx.send(ES::AppState(AppState::Authing));
                                            }
                                        }
                                    }
                                }
                            }
                        });
                    }
                    AppState::Authing => {
                        self.ctx.borrow().ptx.send(PlayReq::Login);
                    }
                    AppState::Offline => {
                        self.last_offline_test = Instant::now();
                        self.ctx.borrow_mut().offline = true;
                    }
                }
            }
            ES::LoginLink(l) => {
                let mut sid = self.ctx.borrow().nc.s_device_id();
                if sid.is_none() {
                    sid.replace(format!(
                        "unknown-{}",
                        fastrand::Rng::new().u32(100000..1000000)
                    ));
                    self.ctx.borrow().nc.set_s_device_id(&sid.as_ref().unwrap());
                }
                self.login_chain = format!(
                    "v1_{}_web_login_{}",
                    sid.unwrap().as_str(),
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map(|v| v.as_millis() as u64)
                        .unwrap_or(1230000000321)
                );
                self.login_qr = Some(
                    QrCode::new(format!(
                        "{}/login?codekey={}&chainId={}",
                        TARGET,
                        l,
                        self.login_chain.as_str()
                    ))
                    .unwrap(),
                );
                self.login_key = l.to_owned();
            }
            ES::SEC => match self.state {
                AppState::Offline => {
                    if Instant::now()
                        .saturating_duration_since(self.last_offline_test.clone())
                        .as_secs()
                        > 3
                    {
                        self.ctx.borrow().tx.send(ES::AppState(
                            if self.ctx.borrow().nc.is_login() {
                                AppState::Authed
                            } else {
                                AppState::Authing
                            },
                        ));
                    }
                    return;
                }
                AppState::Authing => match self.login_state {
                    LoginState::Failed | LoginState::Ok => {}
                    _ => {
                        if self.login_qr.is_some() {
                            self.ctx.borrow().ptx.send(PlayReq::WatchLogin(
                                self.login_key.to_owned(),
                                self.login_chain.to_owned(),
                            ));
                        }
                    }
                },
                _ => {}
            },
            _ => {}
        }

        if self.head.event(e) && self.slide.event(e) {
            self.content.event(e);
        }
        self.footer.event(e);
    }
}
