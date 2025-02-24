use std::error::Error;

pub mod sqlite_storage;

// Define the storage trait for persisting workflow state.
#[async_trait::async_trait]
pub trait WorkflowStorage: Send + Sync {
    async fn init(&self) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn create_task_record(&self, workflow_id: &str, task_id: &str)
        -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn update_task_state(
        &self,
        task_id: &str,
        state: &str,
        attempts: usize,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn get_workflow_status(
        &self,
        workflow_id: &str,
    ) -> Result<Vec<(String, String, usize)>, Box<dyn Error + Send + Sync>>;
}
