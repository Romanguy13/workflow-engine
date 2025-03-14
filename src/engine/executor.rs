use log::{error, info};
use std::sync::Arc;
use tokio::time::sleep;

use super::execution::WorkflowSharedState;
use super::task_state::TaskState;
use super::task_wrapper::TaskWrapper;
use crate::storage::WorkflowStorage;

pub struct TaskExecutor {
    shared_state: WorkflowSharedState,
}

impl TaskExecutor {
    pub fn new(shared_state: WorkflowSharedState) -> Self {
        Self { shared_state }
    }

    pub fn spawn_task(&self, task_id: String) {
        let shared_state = self.shared_state.clone();

        tokio::spawn(async move {
            // Get task details
            let wrapper = match shared_state.tasks.get(&task_id) {
                Some(w) => w.clone(),
                None => {
                    error!("Task '{}' not found in workflow", task_id);
                    return;
                }
            };

            // Execute task with retry logic
            let success =
                Self::execute_task_with_retry(&task_id, &wrapper, &shared_state.storage).await;

            // Signal completion (whether successful or not)
            if let Err(e) = shared_state.completion_sender.send(task_id.clone()) {
                error!("Failed to signal completion for task '{}': {}", task_id, e);
                return;
            }

            // Process dependent tasks
            Self::process_dependent_tasks(task_id, shared_state, success).await;
        });
    }

    async fn execute_task_with_retry(
        task_id: &str,
        wrapper: &TaskWrapper,
        storage: &Arc<dyn WorkflowStorage>,
    ) -> bool {
        let mut attempts = 0;
        let max_attempts = wrapper.retry_policy.max_retries + 1; // including the first attempt

        loop {
            attempts += 1;
            info!(
                "Task '{}' starting (attempt {}/{})",
                task_id, attempts, max_attempts
            );

            // Update task state to "running"
            if let Err(e) = storage
                .update_task_state(task_id, TaskState::Running, attempts)
                .await
            {
                error!("Error updating state for task '{}': {}", task_id, e);
            }

            // Execute the task
            match wrapper.task.execute().await {
                Ok(_) => {
                    info!("Task '{}' completed successfully", task_id);
                    if let Err(e) = storage
                        .update_task_state(task_id, TaskState::Completed, attempts)
                        .await
                    {
                        error!("Error updating state for task '{}': {}", task_id, e);
                    }
                    return true; // Task succeeded
                }
                Err(e) => {
                    error!("Task '{}' failed on attempt {}: {}", task_id, attempts, e);

                    // Check if retries remain
                    if attempts < max_attempts {
                        info!(
                            "Retrying task '{}' after {:?}",
                            task_id, wrapper.retry_policy.retry_delay
                        );
                        sleep(wrapper.retry_policy.retry_delay).await;
                    } else {
                        // All retry attempts exhausted
                        error!("Task '{}' failed after {} attempts", task_id, attempts);
                        if let Err(e) = storage
                            .update_task_state(task_id, TaskState::Failed, attempts)
                            .await
                        {
                            error!("Error updating state for task '{}': {}", task_id, e);
                        }
                        return false; // Task failed
                    }
                }
            }
        }
    }

    async fn process_dependent_tasks(
        completed_task_id: String,
        shared_state: WorkflowSharedState,
        success: bool,
    ) {
        // First, if this task failed, we need to mark all its dependents as failed
        // This includes direct dependents and all their downstream tasks
        if !success {
            Self::mark_downstream_tasks_as_failed(&completed_task_id, &shared_state).await;
            return;
        }

        let mut dependent_tasks_ready = Vec::new();

        // Find dependent tasks that are now ready
        if let Some(dependent_tasks) = shared_state.dependents.get(&completed_task_id) {
            for dependent_id in dependent_tasks {
                let is_ready = {
                    let mut degree_lock = shared_state.in_degree.lock().unwrap();
                    if let Some(count) = degree_lock.get_mut(dependent_id) {
                        *count -= 1;
                        *count == 0 // Task is ready if count reaches zero
                    } else {
                        false
                    }
                };

                if is_ready {
                    dependent_tasks_ready.push(dependent_id.clone());
                }
            }
        }

        // Create executor and spawn all ready dependent tasks
        let executor = TaskExecutor::new(shared_state);
        for task_id in dependent_tasks_ready {
            executor.spawn_task(task_id);
        }
    }

    async fn mark_downstream_tasks_as_failed(
        failed_task_id: &str,
        shared_state: &WorkflowSharedState,
    ) {
        if let Some(dependent_tasks) = shared_state.dependents.get(failed_task_id) {
            for dependent_id in dependent_tasks {
                // Mark the current dependent as failed
                info!(
                    "Marking task '{}' as failed because it depends on failed task '{}'",
                    dependent_id, failed_task_id
                );

                if let Err(e) = shared_state
                    .storage
                    .update_task_state(dependent_id, TaskState::Failed, 0)
                    .await
                {
                    error!(
                        "Error updating state for dependent task '{}': {}",
                        dependent_id, e
                    );
                }

                // Also send a completion signal for this task since it won't be executed
                let _ = shared_state.completion_sender.send(dependent_id.clone());

                // Use Box::pin for recursive async call
                Box::pin(Self::mark_downstream_tasks_as_failed(
                    dependent_id,
                    shared_state,
                ))
                .await;
            }
        }
    }
}
