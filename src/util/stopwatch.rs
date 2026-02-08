use std::time::{Duration, Instant};

pub struct Stopwatch {
    start_time: Option<Instant>,
    elapsed: Duration,
}

impl Stopwatch {
    pub fn new() -> Self {
        Stopwatch {
            start_time: None,
            elapsed: Duration::from_secs(0),
        }
    }

    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
    }

    pub fn stop(&mut self) -> Duration {
        if let Some(start) = self.start_time {
            let elapsed_since_start = start.elapsed();
            self.elapsed += elapsed_since_start;
            self.start_time = None;
            elapsed_since_start
        } else {
            Duration::from_secs(0)
        }
    }

    pub fn elapsed(&self) -> Duration {
        let mut total = self.elapsed;
        if let Some(start) = self.start_time {
            total += start.elapsed();
        }
        total
    }

    pub fn reset(&mut self) {
        self.start_time = None;
        self.elapsed = Duration::from_secs(0);
    }

    pub fn is_running(&self) -> bool {
        self.start_time.is_some()
    }
}
