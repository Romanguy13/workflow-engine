use std::time::Duration;

#[derive(Clone)]
pub struct RetryPolicy {
    pub max_retries: usize,
    pub retry_delay: Duration,
}

impl RetryPolicy {
    pub fn default() -> Self {
        Self {
            max_retries: 0,
            retry_delay: Duration::from_secs(0),
        }
    }

    pub fn new(max_retries: usize, retry_delay: Duration) -> Self {
        Self {
            max_retries,
            retry_delay,
        }
    }

    pub fn with_max_retries(mut self, value: usize) -> Self {
        self.max_retries = value;
        self
    }

    pub fn with_retry_delay(mut self, value: Duration) -> Self {
        self.retry_delay = value;
        self
    }
}
