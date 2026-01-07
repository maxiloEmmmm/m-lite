use std::{
    sync::{Arc, mpsc},
    time::Instant,
};

use ratatui::crossterm::event::{Event, KeyCode, KeyEventKind};
use tokio_util::sync::CancellationToken;

use crate::{event::ES, play::PlayCtx, ui::app::App};

mod config;
mod event;
mod m163;
mod play;
mod ui;

fn main() {
    color_eyre::install().unwrap();

    let mut c = config::load();
    let tc = CancellationToken::new();
    let (event_tx, event_rx) = mpsc::channel::<ES>();
    let nn = Arc::new(m163::client::Nc::new(event_tx.clone(), c.clone()).unwrap());
    let mut terminal = ratatui::init();

    let stream_handle =
        rodio::OutputStreamBuilder::open_default_stream().expect("open audio stream failed");
    let sink = rodio::Sink::connect_new(stream_handle.mixer());
    sink.set_volume(c.volume);
    let (task, play_tx) = play::play(
        PlayCtx {
            nc: nn.clone(),
            event_tx: event_tx.clone(),
            cancel: tc.clone(),
            config: c.clone(),
        },
        sink,
    );
    let mut app = App::new(nn.clone(), event_tx.clone(), play_tx.clone(), c);
    app.ctx.borrow().rt.spawn(task);

    app.ctx
        .borrow()
        .rt
        .spawn(event::ui_event_loop(event_tx.clone(), tc.clone()));

    event_tx.send(ES::AppState(app.state.clone()));
    let mut find = false;
    'top: loop {
        find = false;
        if let Err(ee) = terminal.draw(|frame: &mut ratatui::Frame| {
            app.draw(frame);
        }) {
            println!("draw {:?}", ee);
            break;
        }

        let st = Instant::now();
        // 只触发了sec
        let mut only_sec = true;
        loop {
            let mut res: ES;
            if !find || only_sec {
                res = event_rx.recv().unwrap();
                find = true;
            } else {
                // 等到了以后多处理一些 event 避免 event 爆发不必要的渲染
                res = match event_rx.try_recv() {
                    Ok(e) => e,
                    Err(ee) => match ee {
                        mpsc::TryRecvError::Empty => {
                            break; // 没等到下一个就直接渲染
                        }
                        mpsc::TryRecvError::Disconnected => break 'top,
                    },
                }
            }
            match &res {
                ES::Event(e) => {
                    only_sec = false;
                    if let Event::Key(key) = e {
                        if !key.kind.eq(&KeyEventKind::Press) {
                            continue;
                        }
                        if KeyCode::Char('q') == key.code {
                            break 'top;
                        }
                    }
                }
                ES::SEC => {}
                _ => {
                    only_sec = false;
                }
            }

            app.event(&mut res);

            // sec 为定时触发,大多数sec没有必要再次触发渲染,需要触发的自己发送 Render 事件
            if !only_sec && st.elapsed().as_millis() > 250 {
                // 如果需要渲染则就等250ms
                break;
            }
        }
    }
    ratatui::restore();
    tc.cancel();
}
