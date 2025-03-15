#[cfg(test)]
mod tests {
    use super::super::super::task_wrapper::TaskWrapper;
    use super::super::validation::validate_workflow_tasks;
    use crate::task::{RetryPolicy, Task};
    use std::collections::HashMap;
    use std::error::Error;
    use std::sync::Arc;

    #[derive(Clone)]
    struct EmptyTask;

    #[async_trait::async_trait]
    impl Task for EmptyTask {
        async fn execute(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
            Ok(())
        }
    }

    fn create_task_wrapper(deps: Vec<String>) -> TaskWrapper {
        TaskWrapper {
            task: Arc::new(EmptyTask),
            dependencies: deps,
            retry_policy: RetryPolicy::default(),
        }
    }

    #[test]
    fn test_validate_workflow_valid() {
        let mut tasks = HashMap::new();
        tasks.insert("task1".to_string(), create_task_wrapper(vec![]));
        tasks.insert(
            "task2".to_string(),
            create_task_wrapper(vec!["task1".to_string()]),
        );
        tasks.insert(
            "task3".to_string(),
            create_task_wrapper(vec!["task1".to_string(), "task2".to_string()]),
        );

        let result = validate_workflow_tasks(&tasks);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_workflow_missing_dependency() {
        let mut tasks = HashMap::new();
        tasks.insert("task1".to_string(), create_task_wrapper(vec![]));
        tasks.insert(
            "task2".to_string(),
            create_task_wrapper(vec!["non_existent".to_string()]),
        );

        let result = validate_workflow_tasks(&tasks);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_workflow_cycle() {
        let mut tasks = HashMap::new();
        tasks.insert(
            "task1".to_string(),
            create_task_wrapper(vec!["task3".to_string()]),
        );
        tasks.insert(
            "task2".to_string(),
            create_task_wrapper(vec!["task1".to_string()]),
        );
        tasks.insert(
            "task3".to_string(),
            create_task_wrapper(vec!["task2".to_string()]),
        );

        let result = validate_workflow_tasks(&tasks);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_workflow_self_dependency() {
        let mut tasks = HashMap::new();
        tasks.insert(
            "task1".to_string(),
            create_task_wrapper(vec!["task1".to_string()]),
        );

        let result = validate_workflow_tasks(&tasks);
        assert!(result.is_err());
    }
}
