use std::sync::Arc;
use std::time::Duration;
use workflow_engine::storage::sqlite_storage::SqliteStorage;
use workflow_engine::storage::WorkflowStorage;
use workflow_engine::task::Task;
use workflow_engine::{RetryPolicy, Workflow, WorkflowOptions};

#[derive(Clone)]
struct DataProcessingTask {
    name: String,
    processing_time: Duration,
    should_fail: bool,
}

#[async_trait::async_trait]
impl Task for DataProcessingTask {
    async fn execute(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("Starting task: {}", self.name);

        // Simulate processing time
        tokio::time::sleep(self.processing_time).await;

        if self.should_fail {
            println!("Task {} failed", self.name);
            Err("Task failure simulated".into())
        } else {
            println!("Task {} completed successfully", self.name);
            Ok(())
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    env_logger::init();
    println!("Starting advanced workflow example...");

    // Create storage
    let storage = Arc::new(
        SqliteStorage::new("sqlite:workflow.db")
            .await
            .expect("Failed to create SqliteStorage"),
    );
    storage.init().await?;

    // Set workflow options
    let options: WorkflowOptions = WorkflowOptions::new().with_max_concurrency(2);

    // Create a workflow with multiple tasks
    let workflow = Workflow::new("advanced_workflow", storage.clone())
        .with_options(options)
        .add_task(
            "ingest",
            DataProcessingTask {
                name: "Data Ingestion".into(),
                processing_time: Duration::from_secs(2),
                should_fail: false,
            },
            vec![],
            None,
        )
        .add_task(
            "validate",
            DataProcessingTask {
                name: "Data Validation".into(),
                processing_time: Duration::from_secs(3),
                should_fail: false,
            },
            vec!["ingest".into()],
            None,
        )
        .add_task(
            "transform",
            DataProcessingTask {
                name: "Data Transformation".into(),
                processing_time: Duration::from_secs(4),
                should_fail: false, // This will fail but retry
            },
            vec!["ingest".into()],
            None,
        )
        .add_task(
            "enrich",
            DataProcessingTask {
                name: "Data Enrichment".into(),
                processing_time: Duration::from_secs(3),
                should_fail: true,
            },
            vec!["ingest".into()],
            Some(RetryPolicy {
                max_retries: 2,
                retry_delay: Duration::from_secs(1),
            }),
        )
        .add_task(
            "aggregate",
            DataProcessingTask {
                name: "Data Aggregation".into(),
                processing_time: Duration::from_secs(5),
                should_fail: false,
            },
            vec!["validate".into(), "transform".into(), "enrich".into()],
            None,
        )
        .add_task(
            "report",
            DataProcessingTask {
                name: "Report Generation".into(),
                processing_time: Duration::from_secs(2),
                should_fail: false,
            },
            vec!["aggregate".into()],
            None,
        );

    println!("Executing workflow with 6 tasks...");
    workflow.execute().await?;

    // Print final status
    let statuses = storage.get_workflow_status("advanced_workflow").await?;
    println!("\nFinal workflow status:");
    for (id, state, attempts) in statuses {
        println!("  Task {}: {} after {} attempt(s)", id, state, attempts);
    }

    Ok(())
}
