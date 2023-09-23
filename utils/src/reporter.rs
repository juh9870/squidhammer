use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, SendError, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Copy, Clone)]
pub struct Reporter<T> {
    current_data: T,
    latest_data: Option<T>,
    pub next_update: Instant,
    update_interval: Duration,
}

impl<T> Reporter<T> {
    pub fn new(initial_data: T, update_interval: Duration) -> Self {
        Self {
            next_update: Instant::now() + update_interval,
            latest_data: None,
            current_data: initial_data,
            update_interval,
        }
    }

    pub fn push(&mut self, data: T) {
        self.latest_data = Some(data);
    }

    pub fn read(&mut self) -> &T {
        if Instant::now() >= self.next_update && self.latest_data.is_some() {
            let latest = std::mem::take(&mut self.latest_data)
                .expect("Data should be present at this point");
            self.current_data = latest;
            self.next_update = Instant::now() + self.update_interval;
        }
        &self.current_data
    }

    pub fn peek_latest(&self) -> &T {
        match &self.latest_data {
            None => &self.current_data,
            Some(data) => data,
        }
    }
}

pub fn report_pair<T, R>(reporter: Reporter<T>) -> (ReportSender<T, R>, ReportReceiver<T, R>) {
    let progress = channel::<T>();
    let result = channel::<R>();
    let done = Arc::new(AtomicBool::default());
    let canceled = Arc::new(AtomicBool::default());
    let receiver = ReportReceiver {
        progress_channel: progress.1,
        result_channel: result.1,
        done_data: None,
        done: done.clone(),
        canceled: canceled.clone(),
        reporter,
    };
    let sender = ReportSender {
        progress_channel: progress.0,
        result_channel: result.0,
        done,
        canceled,
    };
    (sender, receiver)
}

#[derive(Debug)]
pub struct ReportReceiver<T, R> {
    progress_channel: Receiver<T>,
    result_channel: Receiver<R>,
    done_data: Option<R>,
    done: Arc<AtomicBool>,
    canceled: Arc<AtomicBool>,
    pub reporter: Reporter<T>,
}

impl<T, R> ReportReceiver<T, R> {
    fn sync(&mut self) {
        for x in self.progress_channel.try_iter() {
            if self.done.load(Ordering::Relaxed) {
                panic!("Progress message was received after Done by ReportReceiver")
            }
            self.reporter.push(x)
        }
        for r in self.result_channel.try_iter() {
            if self.done.load(Ordering::Relaxed) {
                panic!("Multiple Done messages were passed to ReportReceiver")
            }
            self.done_data = Some(r);
        }
    }

    pub fn progress(&mut self) -> &T {
        self.sync();
        self.reporter.read()
    }

    pub fn done(&mut self) -> Option<R> {
        self.done_data.as_ref()?;
        std::mem::take(&mut self.done_data)
    }

    pub fn cancel(&self) {
        self.canceled.swap(true, Ordering::Relaxed);
    }
    pub fn canceled(&self) -> bool {
        self.canceled.load(Ordering::Relaxed)
    }
}

// Can't be cloned on purpose, in order to make Done message consume the sender
#[derive(Debug)]
pub struct ReportSender<T, R> {
    progress_channel: Sender<T>,
    result_channel: Sender<R>,
    done: Arc<AtomicBool>,
    canceled: Arc<AtomicBool>,
}

impl<T, R> ReportSender<T, R> {
    /// Sends a progress message to the receiver
    ///
    /// # Panics
    ///
    /// Panics if `done` payload was already passed. Can only happen as a
    /// result of usage of [ReportSender::clone_unchecked]
    pub fn progress(&self, data: T) -> Result<(), SendError<T>> {
        if self.done.load(Ordering::Relaxed) {
            panic!("Progress message can't be passed after Done");
        }
        self.progress_channel.send(data)
    }

    /// Sends a done payload to the receiver
    ///
    /// # Panics
    ///
    /// Panics if `done` message was already passed. Can only happen as a
    /// result of usage of [ReportSender::clone_unchecked]
    pub fn done(self, data: R) -> Result<(), SendError<R>> {
        if self.done.load(Ordering::Relaxed) {
            panic!("Done message can't be passed more than once");
        }
        self.result_channel.send(data)
    }

    /// Indicator for the reporter that receiving side has canceled listening
    ///
    /// This serves merely as a message and isn't checked in any other methods,
    /// but it's probably a good idea to check for this on every iteration of
    /// processing loop
    pub fn canceled(&self) -> bool {
        self.canceled.load(Ordering::Relaxed)
    }

    /// Creates a clone of this receiver
    ///
    /// # Warning
    /// By using this method you must manually ensure that `done` message
    /// doesn't get passed twice, otherwise panics may arise
    pub fn clone_unchecked(&self) -> ReportSender<T, R> {
        Self {
            progress_channel: self.progress_channel.clone(),
            result_channel: self.result_channel.clone(),
            done: self.done.clone(),
            canceled: self.canceled.clone(),
        }
    }
}
