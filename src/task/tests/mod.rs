#[cfg(test)]
mod tests {
    use crate::RetryPolicy;
    use crate::Task;
    use std::error::Error;
    use std::time::Duration;

    // Test RetryPolicy
    #[test]
    fn test_retry_policy_default() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_retries, 0);
        assert_eq!(policy.retry_delay.as_secs(), 0);
    }

    #[test]
    fn test_retry_policy_builder() {
        let policy = RetryPolicy::default()
            .with_max_retries(3)
            .with_retry_delay(Duration::from_secs(2));

        assert_eq!(policy.max_retries, 3);
        assert_eq!(policy.retry_delay.as_secs(), 2);
    }

    // Test Task implementation
    #[derive(Clone)]
    struct TestTask {
        should_fail: bool,
    }

    #[async_trait::async_trait]
    impl Task for TestTask {
        async fn execute(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
            if self.should_fail {
                Err("Task failed".into())
            } else {
                Ok(())
            }
        }
    }

    #[tokio::test]
    async fn test_task_execution_success() {
        let task = TestTask { should_fail: false };
        let result = task.execute().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_task_execution_failure() {
        let task = TestTask { should_fail: true };
        let result = task.execute().await;
        assert!(result.is_err());
    }
}
