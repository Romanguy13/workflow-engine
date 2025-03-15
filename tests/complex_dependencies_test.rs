use async_trait::async_trait;
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use workflow_engine::storage::implementations::MemoryStorage;
use workflow_engine::{Task, TaskState, Workflow, WorkflowStorage};

// Define a test task that tracks execution order
#[derive(Clone)]
struct TestTask {
    name: String,
    should_fail: bool,
    executed: Arc<std::sync::atomic::AtomicBool>,
    exec_time: Arc<Mutex<Option<std::time::Instant>>>,
    execution_order: Arc<Mutex<Vec<String>>>,
}

impl TestTask {
    fn new(name: &str, should_fail: bool, execution_order: Arc<Mutex<Vec<String>>>) -> Self {
        Self {
            name: name.to_string(),
            should_fail,
            executed: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            exec_time: Arc::new(Mutex::new(None)),
            execution_order,
        }
    }

    fn was_executed(&self) -> bool {
        self.executed.load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[async_trait]
impl Task for TestTask {
    async fn execute(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Record execution time
        {
            let mut time = self.exec_time.lock().unwrap();
            *time = Some(std::time::Instant::now());
        }

        // Mark as executed and record in execution order
        self.executed
            .store(true, std::sync::atomic::Ordering::SeqCst);
        {
            let mut order = self.execution_order.lock().unwrap();
            order.push(self.name.clone());
        }

        // Small delay to simulate work
        tokio::time::sleep(Duration::from_millis(10)).await;

        if self.should_fail {
            Err(format!("Task {} failed deliberately", self.name).into())
        } else {
            Ok(())
        }
    }
}

#[tokio::test]
async fn test_diamond_dependency_pattern() {
    // Setup storage
    let storage = Arc::new(MemoryStorage::new());

    // Track execution order
    let execution_order = Arc::new(Mutex::new(Vec::new()));

    /*
             A
           /   \
          B     C
           \   /
             D
    */

    let task_a = TestTask::new("A", false, execution_order.clone());
    let task_b = TestTask::new("B", false, execution_order.clone());
    let task_c = TestTask::new("C", false, execution_order.clone());
    let task_d = TestTask::new("D", false, execution_order.clone());

    let workflow = Workflow::new("diamond_workflow", storage.clone())
        .add_task("A", task_a.clone(), vec![], None)
        .add_task("B", task_b.clone(), vec!["A".to_string()], None)
        .add_task("C", task_c.clone(), vec!["A".to_string()], None)
        .add_task(
            "D",
            task_d.clone(),
            vec!["B".to_string(), "C".to_string()],
            None,
        );

    let result = workflow.execute().await;
    assert!(result.is_ok());

    // All tasks should have executed
    assert!(task_a.was_executed());
    assert!(task_b.was_executed());
    assert!(task_c.was_executed());
    assert!(task_d.was_executed());

    // Check execution order constraints
    let order = execution_order.lock().unwrap();

    // A must be first
    assert_eq!(order[0], "A");

    // D must be last
    assert_eq!(order[order.len() - 1], "D");

    // B and C can be in any order, but both must be after A and before D
    let b_pos = order.iter().position(|x| x == "B").unwrap();
    let c_pos = order.iter().position(|x| x == "C").unwrap();
    let d_pos = order.iter().position(|x| x == "D").unwrap();

    assert!(b_pos > 0);
    assert!(c_pos > 0);
    assert!(b_pos < d_pos);
    assert!(c_pos < d_pos);

    // Check statuses in storage
    let statuses = storage
        .get_workflow_status("diamond_workflow")
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
async fn test_complex_multi_level_dependencies() {
    // Setup storage
    let storage = Arc::new(MemoryStorage::new());

    // Track execution order
    let execution_order = Arc::new(Mutex::new(Vec::new()));

    /*
          A
         / \
        B   C
       /|   |\
      D E   F G
       \|  /|/
        H  I
         \/
         J
    */

    let tasks = [
        ("A", vec![]),
        ("B", vec!["A"]),
        ("C", vec!["A"]),
        ("D", vec!["B"]),
        ("E", vec!["B"]),
        ("F", vec!["C"]),
        ("G", vec!["C"]),
        ("H", vec!["D", "E"]),
        ("I", vec!["F", "G"]),
        ("J", vec!["H", "I"]),
    ];

    // Create tasks
    let task_map: HashMap<_, _> = tasks
        .iter()
        .map(|(name, _)| {
            (
                name.to_string(),
                TestTask::new(name, false, execution_order.clone()),
            )
        })
        .collect();

    // Build workflow
    let mut workflow = Workflow::new("multi_level_workflow", storage.clone());

    for (name, deps) in tasks.iter() {
        let task = task_map.get(*name).unwrap().clone();
        let dependencies = deps.iter().map(|d| d.to_string()).collect();
        workflow = workflow.add_task(*name, task, dependencies, None);
    }

    let result = workflow.execute().await;
    assert!(result.is_ok());

    // All tasks should have executed
    for (name, _) in tasks.iter() {
        assert!(
            task_map.get(*name).unwrap().was_executed(),
            "Task {} should have executed",
            name
        );
    }

    // Check execution order constraints
    let order = execution_order.lock().unwrap();

    // Check that dependencies were respected
    let positions: HashMap<_, _> = order
        .iter()
        .enumerate()
        .map(|(pos, name)| (name.clone(), pos))
        .collect();

    for (name, deps) in tasks.iter() {
        let task_pos = *positions.get(*name).unwrap();

        for dep in deps {
            let dep_pos = *positions.get(*dep).unwrap();
            assert!(
                dep_pos < task_pos,
                "Task {} should execute after its dependency {}",
                name,
                dep
            );
        }
    }

    // Check that final task J is last
    assert_eq!(order.last().unwrap(), "J");
}

#[tokio::test]
async fn test_fan_out_fan_in_pattern() {
    // Setup storage
    let storage = Arc::new(MemoryStorage::new());

    // Track execution order
    let execution_order = Arc::new(Mutex::new(Vec::new()));

    /*
          Start
        /  |  \  \
       /   |   \  \
      A    B    C  D
       \   |   /  /
        \  |  /  /
           End
    */

    let task_start = TestTask::new("Start", false, execution_order.clone());
    let task_a = TestTask::new("A", false, execution_order.clone());
    let task_b = TestTask::new("B", false, execution_order.clone());
    let task_c = TestTask::new("C", false, execution_order.clone());
    let task_d = TestTask::new("D", false, execution_order.clone());
    let task_end = TestTask::new("End", false, execution_order.clone());

    let workflow = Workflow::new("fan_workflow", storage.clone())
        .add_task("Start", task_start.clone(), vec![], None)
        .add_task("A", task_a.clone(), vec!["Start".to_string()], None)
        .add_task("B", task_b.clone(), vec!["Start".to_string()], None)
        .add_task("C", task_c.clone(), vec!["Start".to_string()], None)
        .add_task("D", task_d.clone(), vec!["Start".to_string()], None)
        .add_task(
            "End",
            task_end.clone(),
            vec![
                "A".to_string(),
                "B".to_string(),
                "C".to_string(),
                "D".to_string(),
            ],
            None,
        );

    let result = workflow.execute().await;
    assert!(result.is_ok());

    // All tasks should have executed
    assert!(task_start.was_executed());
    assert!(task_a.was_executed());
    assert!(task_b.was_executed());
    assert!(task_c.was_executed());
    assert!(task_d.was_executed());
    assert!(task_end.was_executed());

    // Check execution order constraints
    let order = execution_order.lock().unwrap();

    // Start should be first
    assert_eq!(order[0], "Start");

    // End should be last
    assert_eq!(order[order.len() - 1], "End");

    // A, B, C, D can be in any order, but all must be after Start and before End
    let start_pos = order.iter().position(|x| x == "Start").unwrap();
    let end_pos = order.iter().position(|x| x == "End").unwrap();

    for task in ["A", "B", "C", "D"].iter() {
        let pos = order.iter().position(|x| x == *task).unwrap();
        assert!(pos > start_pos, "Task {} should execute after Start", task);
        assert!(pos < end_pos, "Task {} should execute before End", task);
    }
}

#[tokio::test]
async fn test_task_failure_in_complex_graph() {
    // Setup storage
    let storage = Arc::new(MemoryStorage::new());

    // Track execution order
    let execution_order = Arc::new(Mutex::new(Vec::new()));

    /*
          A
         / \
        B   C (fails)
       /     \
      D       E
    */

    let task_a = TestTask::new("A", false, execution_order.clone());
    let task_b = TestTask::new("B", false, execution_order.clone());
    let task_c = TestTask::new("C", true, execution_order.clone()); // This task will fail
    let task_d = TestTask::new("D", false, execution_order.clone());
    let task_e = TestTask::new("E", false, execution_order.clone()); // Should not execute

    let workflow = Workflow::new("failure_workflow", storage.clone())
        .add_task("A", task_a.clone(), vec![], None)
        .add_task("B", task_b.clone(), vec!["A".to_string()], None)
        .add_task("C", task_c.clone(), vec!["A".to_string()], None)
        .add_task("D", task_d.clone(), vec!["B".to_string()], None)
        .add_task("E", task_e.clone(), vec!["C".to_string()], None);

    let result = workflow.execute().await;
    assert!(result.is_ok());

    // Check which tasks executed
    assert!(task_a.was_executed());
    assert!(task_b.was_executed());
    assert!(task_c.was_executed()); // This failed but still executed
    assert!(task_d.was_executed()); // Should execute because it depends on B which succeeded
    assert!(!task_e.was_executed()); // Should NOT execute because it depends on C which failed

    // Check task states in storage
    let statuses = storage
        .get_workflow_status("failure_workflow")
        .await
        .unwrap();

    let mut status_map = HashMap::new();
    for (id, state, _) in statuses {
        status_map.insert(id, state);
    }

    assert_eq!(status_map.get("A").unwrap(), &TaskState::Completed);
    assert_eq!(status_map.get("B").unwrap(), &TaskState::Completed);
    assert_eq!(status_map.get("C").unwrap(), &TaskState::Failed);
    assert_eq!(status_map.get("D").unwrap(), &TaskState::Completed);
    assert_eq!(status_map.get("E").unwrap(), &TaskState::Failed); // Marked as failed without executing
}

#[tokio::test]
async fn test_dependency_levels_and_execution_timing() {
    // Setup storage
    let storage = Arc::new(MemoryStorage::new());

    // Track execution times
    let execution_times = Arc::new(Mutex::new(HashMap::new()));
    let execution_order = Arc::new(Mutex::new(Vec::new()));

    // Define timing-aware tasks
    struct TimingTask {
        name: String,
        delay: Duration,
        execution_times: Arc<Mutex<HashMap<String, std::time::Instant>>>,
        execution_order: Arc<Mutex<Vec<String>>>,
    }

    impl TimingTask {
        fn new(
            name: &str,
            delay: u64,
            times: Arc<Mutex<HashMap<String, std::time::Instant>>>,
            order: Arc<Mutex<Vec<String>>>,
        ) -> Self {
            Self {
                name: name.to_string(),
                delay: Duration::from_millis(delay),
                execution_times: times,
                execution_order: order,
            }
        }
    }

    #[async_trait]
    impl Task for TimingTask {
        async fn execute(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
            let start = std::time::Instant::now();

            // Record execution start time and order
            {
                let mut times = self.execution_times.lock().unwrap();
                times.insert(self.name.clone(), start);

                let mut order = self.execution_order.lock().unwrap();
                order.push(self.name.clone());
            }

            // Simulate work
            tokio::time::sleep(self.delay).await;

            Ok(())
        }
    }

    // Create tasks with different execution times
    let tasks = [
        ("A", 10), // Level 1: 10ms
        ("B", 50), // Level 2: 50ms
        ("C", 20), // Level 2: 20ms
        ("D", 5),  // Level 3: 5ms
        ("E", 30), // Level 3: 30ms
    ];

    let mut workflow = Workflow::new("timing_workflow", storage.clone());

    for (name, delay) in &tasks {
        let task = TimingTask::new(
            name,
            *delay,
            execution_times.clone(),
            execution_order.clone(),
        );
        workflow = match *name {
            "A" => workflow.add_task(*name, task, vec![], None),
            "B" | "C" => workflow.add_task(*name, task, vec!["A".to_string()], None),
            "D" => workflow.add_task(*name, task, vec!["B".to_string()], None),
            "E" => workflow.add_task(*name, task, vec!["C".to_string()], None),
            _ => workflow,
        };
    }

    let start = std::time::Instant::now();
    let result = workflow.execute().await;
    assert!(result.is_ok());

    // Check execution times
    let times = execution_times.lock().unwrap();
    let order = execution_order.lock().unwrap();

    // A should be first
    assert_eq!(order[0], "A");

    // B and C can start after A
    assert!(times.get("B").unwrap() > times.get("A").unwrap());
    assert!(times.get("C").unwrap() > times.get("A").unwrap());

    // D must start after B
    assert!(times.get("D").unwrap() > times.get("B").unwrap());

    // E must start after C
    assert!(times.get("E").unwrap() > times.get("C").unwrap());

    // Total execution time should be less than sum of all task times
    // (proving parallel execution of some tasks)
    let total_task_time: u64 = tasks.iter().map(|(_, delay)| *delay).sum();
    assert!(start.elapsed() < Duration::from_millis(total_task_time));
}
