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
