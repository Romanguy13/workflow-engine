use async_trait::async_trait;
use std::error::Error;

pub mod retry;
pub use retry::policy::RetryPolicy;

#[async_trait]
pub trait Task: Send + Sync {
    async fn execute(&self) -> Result<(), Box<dyn Error + Send + Sync>>;
}
