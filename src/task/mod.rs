use async_trait::async_trait;
use std::{error::Error, time::Duration};

pub mod example_task;

#[async_trait]
pub trait Task: Send + Sync {
    async fn execute(&self) -> Result<(), Box<dyn Error + Send + Sync>>;
}

/// A struct to define retry policy for tasks.
#[derive(Clone)]
pub struct RetryPolicy {
    pub max_retries: usize,
    pub retry_delay: std::time::Duration,
}

impl RetryPolicy {
    pub fn default() -> Self {
        Self {
            max_retries: 0,
            retry_delay: Duration::from_secs(0),
        }
    }
}
