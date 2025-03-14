use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

use super::task_wrapper::TaskWrapper;
use crate::storage::WorkflowStorage;

pub(crate) struct ExecutionEnvironment {
    /// Total number of tasks in the workflow
    pub task_count: usize,
    /// Shared state for task execution
    pub shared_state: WorkflowSharedState,
    /// Channel receiver for task completion notifications
    pub completion_channel: mpsc::UnboundedReceiver<String>,
}

#[derive(Clone)]
pub(crate) struct WorkflowSharedState {
    /// Map of task IDs to task wrappers
    pub tasks: Arc<HashMap<String, TaskWrapper>>,
    /// Number of dependencies remaining for each task
    pub in_degree: Arc<Mutex<HashMap<String, usize>>>,
    /// Tasks that depend on each task
    pub dependents: Arc<HashMap<String, Vec<String>>>,
    /// Channel sender for task completion notifications
    pub completion_sender: mpsc::UnboundedSender<String>,
    /// Storage backend for persisting task state
    pub storage: Arc<dyn WorkflowStorage>,
}