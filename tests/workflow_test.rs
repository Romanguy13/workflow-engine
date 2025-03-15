use async_trait::async_trait;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use workflow_engine::storage::implementations::MemoryStorage;
use workflow_engine::{Task, TaskState, Workflow, WorkflowStorage};

// Define a test task
#[derive(Clone)]
struct TestTask {
    name: String,
    should_fail: bool,
    executed: Arc<std::sync::atomic::AtomicBool>,
}

impl TestTask {
    fn new(name: &str, should_fail: bool) -> Self {
        Self {
            name: name.to_string(),
            should_fail,
            executed: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    fn was_executed(&self) -> bool {
        self.executed.load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[async_trait]
impl Task for TestTask {
    async fn execute(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.executed
            .store(true, std::sync::atomic::Ordering::SeqCst);

        if self.should_fail {
            Err(format!("Task {} failed deliberately", self.name).into())
        } else {
            tokio::time::sleep(Duration::from_millis(10)).await;
            Ok(())
        }
    }
}

#[tokio::test]
async fn test_linear_workflow() {
    // Setup storage
    let storage = Arc::new(MemoryStorage::new());

    // Create tasks with tracking
    let task1 = TestTask::new("First Task", false);
    let task2 = TestTask::new("Second Task", false);
    let task3 = TestTask::new("Third Task", false);

    // Store references for checking execution
    let task1_ref = task1.clone();
    let task2_ref = task2.clone();
    let task3_ref = task3.clone();

    // Build a simple linear workflow
    let workflow = Workflow::new("linear_workflow", storage.clone())
        .add_task("task1", task1, vec![], None)
        .add_task("task2", task2, vec!["task1".to_string()], None)
        .add_task("task3", task3, vec!["task2".to_string()], None);

    // Execute workflow
    let result = workflow.execute().await;
    assert!(result.is_ok());

    // Check that all tasks executed in order
    assert!(task1_ref.was_executed());
    assert!(task2_ref.was_executed());
    assert!(task3_ref.was_executed());

    // Verify through storage
    let statuses = storage
        .get_workflow_status("linear_workflow")
        .await
        .unwrap();

    for (id, state, _) in statuses {
        assert_eq!(
            state,
            TaskState::Completed,
            "Task {} should be completed",
            id
        );
    }
}

#[tokio::test]
async fn test_parallel_workflow() {
    // Setup storage
    let storage = Arc::new(MemoryStorage::new());

    // Create tasks
    let task_start = TestTask::new("Start Task", false);
    let task_a = TestTask::new("Branch A", false);
    let task_b = TestTask::new("Branch B", false);
    let task_c = TestTask::new("Branch C", false);
    let task_end = TestTask::new("End Task", false);

    // Store references
    let task_start_ref = task_start.clone();
    let task_a_ref = task_a.clone();
    let task_b_ref = task_b.clone();
    let task_c_ref = task_c.clone();
    let task_end_ref = task_end.clone();

    // Build a workflow with parallel branches
    let workflow = Workflow::new("parallel_workflow", storage.clone())
        .add_task("start", task_start, vec![], None)
        .add_task("branch_a", task_a, vec!["start".to_string()], None)
        .add_task("branch_b", task_b, vec!["start".to_string()], None)
        .add_task("branch_c", task_c, vec!["start".to_string()], None)
        .add_task(
            "end",
            task_end,
            vec![
                "branch_a".to_string(),
                "branch_b".to_string(),
                "branch_c".to_string(),
            ],
            None,
        );

    // Execute workflow
    let result = workflow.execute().await;
    assert!(result.is_ok());

    // Check that all tasks executed
    assert!(task_start_ref.was_executed());
    assert!(task_a_ref.was_executed());
    assert!(task_b_ref.was_executed());
    assert!(task_c_ref.was_executed());
    assert!(task_end_ref.was_executed());
}
