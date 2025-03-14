use crate::task::{RetryPolicy, Task};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct TaskWrapper {
    /// The actual task implementation
    pub task: Arc<dyn Task>,
    /// IDs of tasks that this task depends on
    pub dependencies: Vec<String>,
    /// Policy for retrying the task on failure
    pub retry_policy: RetryPolicy,
}
