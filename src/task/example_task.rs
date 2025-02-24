use crate::task::Task;
use async_trait::async_trait;
use std::error::Error;

/// A simple example task to demonstrate the Task trait.
pub struct ExampleTask {
    pub name: String,
    pub fail_once: bool,
}

#[async_trait]
impl Task for ExampleTask {
    async fn execute(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        println!("Executing {}", self.name);
        // Simulate some asynchronous work.
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        if self.fail_once {
            eprintln!("Simulated failure in {}", self.name);
            Err("Simulated error".into())
        } else {
            println!("Finished {}", self.name);
            Ok(())
        }
    }
}
