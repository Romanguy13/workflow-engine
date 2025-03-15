use async_trait::async_trait;
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};

use crate::engine::TaskState;
use crate::storage::WorkflowStorage;
// use crate::Task;

/// In-memory implementation of WorkflowStorage for testing
#[derive(Clone)]
pub struct MemoryStorage {
    task_states: Arc<Mutex<HashMap<String, (String, TaskState, usize)>>>,
    update_calls: Arc<Mutex<Vec<(String, TaskState, usize)>>>,
}

impl MemoryStorage {
    /// Create a new empty memory storage
    pub fn new() -> Self {
        Self {
            task_states: Arc::new(Mutex::new(HashMap::new())),
            update_calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Get the current state of a task
    pub fn get_task_state(&self, task_id: &str) -> Option<(TaskState, usize)> {
        self.task_states
            .lock()
            .unwrap()
            .get(task_id)
            .map(|(_, state, attempts)| (*state, *attempts))
    }

    /// Get all update calls made to this storage
    pub fn get_update_calls(&self) -> Vec<(String, TaskState, usize)> {
        self.update_calls.lock().unwrap().clone()
    }
}

#[async_trait]
impl WorkflowStorage for MemoryStorage {
    async fn init(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Nothing to initialize for in-memory storage
        Ok(())
    }

    async fn create_task_record(
        &self,
        workflow_id: &str,
        task_id: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut states = self.task_states.lock().unwrap();
        states.insert(
            task_id.to_string(),
            (workflow_id.to_string(), TaskState::Pending, 0),
        );
        Ok(())
    }

    async fn update_task_state(
        &self,
        task_id: &str,
        state: TaskState,
        attempts: usize,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Record the update call
        {
            let mut calls = self.update_calls.lock().unwrap();
            calls.push((task_id.to_string(), state, attempts));
        }

        // Update the state
        let mut states = self.task_states.lock().unwrap();
        if let Some((workflow_id, _, _)) = states.get(task_id) {
            let workflow_id = workflow_id.clone();
            states.insert(task_id.to_string(), (workflow_id, state, attempts));
            Ok(())
        } else {
            Err("Task not found".into())
        }
    }

    async fn get_workflow_status(
        &self,
        workflow_id: &str,
    ) -> Result<Vec<(String, TaskState, usize)>, Box<dyn Error + Send + Sync>> {
        let states = self.task_states.lock().unwrap();
        let result = states
            .iter()
            .filter(|(_, (wid, _, _))| wid == workflow_id)
            .map(|(id, (_, state, attempts))| {
                (
                    id.clone(),
                    TaskState::from_str(&state.as_str()).unwrap_or(TaskState::Failed),
                    *attempts,
                )
            })
            .collect();
        Ok(result)
    }
}
