use async_trait::async_trait;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use workflow_engine::storage::implementations::MemoryStorage;
use workflow_engine::{RetryPolicy, Task, TaskState, Workflow, WorkflowStorage};

#[tokio::test]
async fn test_retry_mechanism() {
    // Setup storage
    let storage = Arc::new(MemoryStorage::new());

    // Create a failing task with an execution counter
    let execution_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let ec_clone = execution_count.clone();

    struct CountingTask {
        counter: Arc<std::sync::atomic::AtomicUsize>,
        should_fail: bool,
    }

    #[async_trait]
    impl Task for CountingTask {
        async fn execute(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
            let count = self
                .counter
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                + 1;

            if self.should_fail && count <= 2 {
                Err(format!("Failing on attempt {}", count).into())
            } else {
                Ok(())
            }
        }
    }

    // Add a task with retry policy
    let retry_task = CountingTask {
        counter: execution_count,
        should_fail: true,
    };

    let retry_policy = RetryPolicy::default()
        .with_max_retries(2)
        .with_retry_delay(Duration::from_millis(10));

    let workflow = Workflow::new("retry_workflow", storage.clone()).add_task(
        "retry_task",
        retry_task,
        vec![],
        Some(retry_policy),
    );

    // Execute workflow
    let result = workflow.execute().await;
    assert!(result.is_ok());

    // Check that it executed exactly 3 times (initial + 2 retries)
    assert_eq!(ec_clone.load(std::sync::atomic::Ordering::SeqCst), 3);

    // Verify final status is completed
    let statuses = storage.get_workflow_status("retry_workflow").await.unwrap();
    let status = statuses.first().unwrap().1.clone();
    assert_eq!(status, TaskState::Completed);
}
