use crate::{app::Event, battery, logging};
use std::{
    sync::mpsc::{self, Sender},
    thread,
    time::Duration,
};

const POLL_INTERVAL: Duration = Duration::from_secs(30);
const RETRY_INTERVAL: Duration = Duration::from_secs(5);

pub(crate) struct Worker {
    refresh_sender: Sender<()>,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    pub(crate) fn start(event_sender: Sender<Event>) -> Self {
        let (refresh_sender, refresh_receiver) = mpsc::channel();
        let thread = thread::spawn(move || {
            loop {
                let reading = battery::read_with_logger(|message| {
                    logging::write(logging::Level::Warn, message)
                });
                let delay = if reading.is_ok() {
                    POLL_INTERVAL
                } else {
                    RETRY_INTERVAL
                };
                if event_sender.send(Event::Reading(reading)).is_err() {
                    break;
                }
                crate::platform::wake();
                match refresh_receiver.recv_timeout(delay) {
                    Ok(()) | Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        });
        Self {
            refresh_sender,
            thread: Some(thread),
        }
    }

    pub(crate) fn refresh(&self) {
        let _ = self.refresh_sender.send(());
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        let (replacement, _) = mpsc::channel();
        drop(std::mem::replace(&mut self.refresh_sender, replacement));
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
