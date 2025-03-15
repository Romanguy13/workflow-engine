#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::engine::execution::WorkflowSharedState;
    use crate::engine::task_state::TaskState;
    use crate::engine::task_wrapper::TaskWrapper;
    use crate::storage::{MemoryStorage, WorkflowStorage};
    use crate::task::{RetryPolicy, Task};
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::error::Error;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tokio::sync::mpsc;

    // Mock task for testing
    #[derive(Clone)]
    struct MockTask {
        name: String,
        should_fail: bool,
        execution_count: Arc<Mutex<usize>>,
        execution_delay: Duration,
    }

    impl MockTask {
        fn new(name: impl Into<String>, should_fail: bool) -> Self {
            Self {
                name: name.into(),
                should_fail,
                execution_count: Arc::new(Mutex::new(0)),
                execution_delay: Duration::from_millis(10),
            }
        }

        fn with_delay(mut self, delay: Duration) -> Self {
            self.execution_delay = delay;
            self
        }

        fn get_execution_count(&self) -> usize {
            *self.execution_count.lock().unwrap()
        }
    }

    #[async_trait]
    impl Task for MockTask {
        async fn execute(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
            // Increment execution counter
            {
                let mut count = self.execution_count.lock().unwrap();
                *count += 1;
            }

            // Simulate work with delay
            tokio::time::sleep(self.execution_delay).await;

            if self.should_fail {
                Err(format!("Task {} failed deliberately", self.name).into())
            } else {
                Ok(())
            }
        }
    }

    // Helper to create a workflow with tasks
    fn create_workflow_with_tasks(
        tasks: Vec<(String, MockTask, Vec<String>, Option<RetryPolicy>)>,
    ) -> (
        TaskExecutor,
        Arc<MemoryStorage>,
        mpsc::UnboundedReceiver<String>,
        HashMap<String, Arc<MockTask>>,
    ) {
        // Create mock storage
        let storage = Arc::new(MemoryStorage::new());

        // Create channels
        let (tx, rx) = mpsc::unbounded_channel();

        // Create task map and track mock tasks for assertions
        let mut task_map = HashMap::new();
        let mut mock_tasks = HashMap::new();
        let mut in_degree_map = HashMap::new();
        let mut dependents_map = HashMap::new();

        // Process each task definition
        for (id, task, dependencies, retry_policy) in tasks {
            // Initialize in-degree
            in_degree_map.insert(id.clone(), dependencies.len());

            // Store task for later assertions
            let task_arc = Arc::new(task);
            mock_tasks.insert(id.clone(), task_arc.clone());

            // Create wrapper
            let wrapper = TaskWrapper {
                task: task_arc,
                dependencies: dependencies.clone(),
                retry_policy: retry_policy.unwrap_or_else(RetryPolicy::default),
            };

            // IMPORTANT: Register task with storage before testing
            let storage_clone = storage.clone();
            let id_clone = id.clone();
            tokio::spawn(async move {
                storage_clone
                    .create_task_record("test_workflow", &id_clone)
                    .await
            });

            // Add to task map
            task_map.insert(id, wrapper);
        }

        // Build dependents map
        for (id, wrapper) in &task_map {
            for dep in &wrapper.dependencies {
                dependents_map
                    .entry(dep.clone())
                    .or_insert_with(Vec::new)
                    .push(id.clone());
            }
        }

        // Create shared state
        let shared_state = WorkflowSharedState {
            tasks: Arc::new(task_map),
            in_degree: Arc::new(Mutex::new(in_degree_map)),
            dependents: Arc::new(dependents_map),
            completion_sender: tx,
            storage: storage.clone(),
        };

        // Create executor
        let executor = TaskExecutor::new(shared_state);

        (executor, storage, rx, mock_tasks)
    }

    #[tokio::test]
    async fn test_successful_task_execution() {
        // Create a task that will succeed
        let task_id = "test_task".to_string();
        let mock_task = MockTask::new("Test Task", false);

        let tasks = vec![(task_id.clone(), mock_task.clone(), vec![], None)];
        let (executor, storage, mut rx, mock_tasks) = create_workflow_with_tasks(tasks);

        // Spawn the task
        executor.spawn_task(task_id.clone());

        // Wait for task completion
        let completed_id = rx.recv().await.expect("Should receive completion");
        assert_eq!(completed_id, task_id);

        // Assert the task was executed once
        assert_eq!(mock_tasks.get(&task_id).unwrap().get_execution_count(), 1);

        // Verify task state in storage
        tokio::time::sleep(Duration::from_millis(50)).await; // Small delay to ensure storage updates
        let state = storage
            .get_task_state(&task_id)
            .expect("Task state should exist");
        assert_eq!(state.0, TaskState::Completed);
        assert_eq!(state.1, 1); // First attempt
    }

    #[tokio::test]
    async fn test_retry_on_failure() {
        // Create a task that will fail but has retries
        let task_id = "failing_task".to_string();
        let mock_task = MockTask::new("Failing Task", true);
        let retry_policy = RetryPolicy::default()
            .with_max_retries(2)
            .with_retry_delay(Duration::from_millis(50));

        let tasks = vec![(
            task_id.clone(),
            mock_task.clone(),
            vec![],
            Some(retry_policy),
        )];
        let (executor, storage, mut rx, mock_tasks) = create_workflow_with_tasks(tasks);

        // Spawn the task
        executor.spawn_task(task_id.clone());

        // Wait for task completion (which will be a failure)
        let completed_id = rx.recv().await.expect("Should receive completion");
        assert_eq!(completed_id, task_id);

        // Assert the task was executed multiple times (3 = initial + 2 retries)
        tokio::time::sleep(Duration::from_millis(200)).await; // Ensure all retries completed
        assert_eq!(mock_tasks.get(&task_id).unwrap().get_execution_count(), 3);

        // Verify final task state in storage
        let state = storage
            .get_task_state(&task_id)
            .expect("Task state should exist");
        assert_eq!(state.0, TaskState::Failed);
        assert_eq!(state.1, 3); // Third attempt
    }

    #[tokio::test]
    async fn test_dependent_task_execution() {
        // Create two tasks with a dependency
        let task1_id = "task1".to_string();
        let task2_id = "task2".to_string();

        let task1 = MockTask::new("Task 1", false);
        let task2 = MockTask::new("Task 2", false);

        let tasks = vec![
            (task1_id.clone(), task1.clone(), vec![], None),
            (
                task2_id.clone(),
                task2.clone(),
                vec![task1_id.clone()],
                None,
            ),
        ];

        let (executor, storage, mut rx, mock_tasks) = create_workflow_with_tasks(tasks);

        // Spawn the first task
        executor.spawn_task(task1_id.clone());

        // Wait for both tasks to complete (task2 should be spawned automatically after task1)
        let mut completed = Vec::new();
        for _ in 0..2 {
            completed.push(rx.recv().await.expect("Should receive completion"));
        }

        // We should have received completions for both tasks
        assert!(completed.contains(&task1_id));
        assert!(completed.contains(&task2_id));

        // Both tasks should have executed once
        assert_eq!(mock_tasks.get(&task1_id).unwrap().get_execution_count(), 1);
        assert_eq!(mock_tasks.get(&task2_id).unwrap().get_execution_count(), 1);

        // Verify final task states
        let state1 = storage
            .get_task_state(&task1_id)
            .expect("Task state should exist");
        let state2 = storage
            .get_task_state(&task2_id)
            .expect("Task state should exist");
        assert_eq!(state1.0, TaskState::Completed);
        assert_eq!(state2.0, TaskState::Completed);
    }

    #[tokio::test]
    async fn test_failed_dependency_marks_downstream_as_failed() {
        // Create a chain of tasks where one fails
        let task1_id = "task1".to_string();
        let task2_id = "task2".to_string();
        let task3_id = "task3".to_string();

        let task1 = MockTask::new("Task 1", false);
        let task2 = MockTask::new("Task 2", true); // This task will fail
        let task3 = MockTask::new("Task 3", false);

        let tasks = vec![
            (task1_id.clone(), task1.clone(), vec![], None),
            (
                task2_id.clone(),
                task2.clone(),
                vec![task1_id.clone()],
                None,
            ),
            (
                task3_id.clone(),
                task3.clone(),
                vec![task2_id.clone()],
                None,
            ),
        ];

        let (executor, storage, mut rx, mock_tasks) = create_workflow_with_tasks(tasks);

        // Spawn the first task
        executor.spawn_task(task1_id.clone());

        // Wait for all tasks to be processed
        let mut completed = Vec::new();
        for _ in 0..3 {
            completed.push(rx.recv().await.expect("Should receive completion"));
        }

        // All tasks should be processed
        assert!(completed.contains(&task1_id));
        assert!(completed.contains(&task2_id));
        assert!(completed.contains(&task3_id));

        // Task1 should execute, Task2 should execute and fail, Task3 should be marked as failed without executing
        assert_eq!(mock_tasks.get(&task1_id).unwrap().get_execution_count(), 1);
        assert_eq!(mock_tasks.get(&task2_id).unwrap().get_execution_count(), 1);
        assert_eq!(mock_tasks.get(&task3_id).unwrap().get_execution_count(), 0); // Never executed

        // Verify final task states
        tokio::time::sleep(Duration::from_millis(50)).await; // Ensure storage updates are complete
        let state1 = storage
            .get_task_state(&task1_id)
            .expect("Task state should exist");
        let state2 = storage
            .get_task_state(&task2_id)
            .expect("Task state should exist");
        let state3 = storage
            .get_task_state(&task3_id)
            .expect("Task state should exist");

        assert_eq!(state1.0, TaskState::Completed);
        assert_eq!(state2.0, TaskState::Failed);
        assert_eq!(state3.0, TaskState::Failed); // Should be marked failed due to dependency
    }

    #[tokio::test]
    async fn test_complex_dependency_graph() {
        // Create a more complex dependency graph
        //    A
        //  /   \
        // B     C
        //  \   /
        //    D

        let task_a = "task_a".to_string();
        let task_b = "task_b".to_string();
        let task_c = "task_c".to_string();
        let task_d = "task_d".to_string();

        let tasks = vec![
            (task_a.clone(), MockTask::new("Task A", false), vec![], None),
            (
                task_b.clone(),
                MockTask::new("Task B", false),
                vec![task_a.clone()],
                None,
            ),
            (
                task_c.clone(),
                MockTask::new("Task C", false),
                vec![task_a.clone()],
                None,
            ),
            (
                task_d.clone(),
                MockTask::new("Task D", false),
                vec![task_b.clone(), task_c.clone()],
                None,
            ),
        ];

        let (executor, storage, mut rx, mock_tasks) = create_workflow_with_tasks(tasks);

        // Spawn the root task
        executor.spawn_task(task_a.clone());

        // Wait for all tasks to complete
        let mut completed = Vec::new();
        for _ in 0..4 {
            completed.push(rx.recv().await.expect("Should receive completion"));
        }

        // All tasks should have executed once
        for (_, mock_task) in &mock_tasks {
            assert_eq!(mock_task.get_execution_count(), 1);
        }

        // Check all tasks completed
        for task_id in &[&task_a, &task_b, &task_c, &task_d] {
            let state = storage
                .get_task_state(task_id)
                .expect("Task state should exist");
            assert_eq!(state.0, TaskState::Completed);
        }
    }

    #[tokio::test]
    async fn test_async_execution_ordering() {
        // Test that tasks execute in the right order based on dependencies
        // even with varying execution times

        let task1_id = "fast_task".to_string();
        let task2_id = "slow_task".to_string();
        let task3_id = "dependent_task".to_string();

        // Create tasks with different execution times
        let fast_task = MockTask::new("Fast Task", false).with_delay(Duration::from_millis(10));
        let slow_task = MockTask::new("Slow Task", false).with_delay(Duration::from_millis(100));
        let dependent_task = MockTask::new("Dependent Task", false);

        let tasks = vec![
            (task1_id.clone(), fast_task.clone(), vec![], None),
            (task2_id.clone(), slow_task.clone(), vec![], None),
            (
                task3_id.clone(),
                dependent_task.clone(),
                vec![task1_id.clone(), task2_id.clone()],
                None,
            ),
        ];

        let (executor, storage, mut rx, _) = create_workflow_with_tasks(tasks);

        // Record start time
        let start = std::time::Instant::now();

        // Spawn both independent tasks
        executor.spawn_task(task1_id.clone());
        executor.spawn_task(task2_id.clone());

        // Wait for all completions
        let mut completed_order = Vec::new();
        for _ in 0..3 {
            completed_order.push(rx.recv().await.expect("Should receive completion"));
        }

        // The dependent task should be the last one completed
        assert_eq!(completed_order.last().unwrap(), &task3_id);

        // The dependent task should only complete after at least 100ms (slow task duration)
        assert!(start.elapsed() >= Duration::from_millis(100));

        // Check all tasks completed
        let state3 = storage
            .get_task_state(&task3_id)
            .expect("Task state should exist");
        assert_eq!(state3.0, TaskState::Completed);
    }
}
