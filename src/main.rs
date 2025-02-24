use crate::storage::WorkflowStorage;
use std::sync::Arc;

mod engine;
mod storage;
mod task;

use engine::Workflow;
use storage::sqlite_storage::SqliteStorage;
use task::example_task::ExampleTask;
use task::RetryPolicy;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logging if needed.
    env_logger::init();
    println!("Starting the open source Rust Workflow Engine...");

    // Step 1: Set up the storage backend.
    let storage = Arc::new(
        SqliteStorage::new("sqlite:workflows.db")
            .await
            .expect("Failed to create SqliteStorage"),
    );
    storage.init().await?;

    // Step 2: Create a new workflow.
    let workflow = Workflow::new("workflow1", storage.clone())
        .add_task(
            "task1",
            ExampleTask {
                name: "Task 1".into(),
                fail_once: false,
            },
            vec![],
            None,
        )
        .add_task(
            "task2",
            ExampleTask {
                name: "Task 2".into(),
                fail_once: true,
            },
            vec!["task1".into()],
            Some(RetryPolicy {
                max_retries: 2,
                retry_delay: std::time::Duration::from_secs(2),
            }),
        )
        .add_task(
            "task3",
            ExampleTask {
                name: "Task 3".into(),
                fail_once: false,
            },
            vec!["task1".into()],
            None,
        )
        .add_task(
            "task4",
            ExampleTask {
                name: "Task 4".into(),
                fail_once: false,
            },
            vec!["task2".into(), "task3".into()],
            None,
        );

    // Step 3: Execute the workflow.
    workflow.execute().await?;

    // Step 4: Optionally, query the workflow status.
    let statuses = storage.get_workflow_status("workflow1").await?;
    println!("Workflow status:");
    for (id, state, attempts) in statuses {
        println!("  Task {}: {} after {} attempt(s)", id, state, attempts);
    }

    // Step 5 (Optional): Start your API server if you are exposing endpoints.
    // tokio::spawn(async move {
    //     start_api(storage.clone()).await;
    // });

    Ok(())
}
