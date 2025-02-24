use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};
use log::info;
use tokio::sync::mpsc;
use tokio::time::sleep;

use crate::storage::WorkflowStorage;
use crate::task::{RetryPolicy, Task};

#[derive(Clone)]
pub struct Workflow {
    tasks: HashMap<String, TaskWrapper>,
    workflow_id: String,
    storage: Arc<dyn WorkflowStorage>,
}

impl Workflow {
    pub fn new(workflow_id: impl Into<String>, storage: Arc<dyn WorkflowStorage>) -> Self {
        Self {
            tasks: HashMap::new(),
            workflow_id: workflow_id.into(),
            storage,
        }
    }

    pub fn add_task<T: Task + 'static>(
        mut self,
        id: impl Into<String>,
        task: T,
        dependencies: Vec<String>,
        retry_policy: Option<RetryPolicy>,
    ) -> Self {
        let id = id.into();
        self.tasks.insert(
            id.clone(),
            TaskWrapper {
                task: Arc::new(task),
                dependencies,
                retry_policy: retry_policy.unwrap_or_else(RetryPolicy::default),
            },
        );
        self
    }

    pub async fn execute(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Ensure all task records are created in storage.
        for task_id in self.tasks.keys() {
            self.storage
                .create_task_record(&self.workflow_id, task_id)
                .await?
        }

        // Compute in-degrees and build a dependency graph.
        let in_degree: HashMap<String, usize> = self
            .tasks
            .iter()
            .map(|(id, wrapper)| (id.clone(), wrapper.dependencies.len()))
            .collect();

        let mut dependents: HashMap<String, Vec<String>> = HashMap::new();
        for (id, wrapper) in &self.tasks {
            for dep in &wrapper.dependencies {
                dependents.entry(dep.clone()).or_default().push(id.clone());
            }
        }

        let (tx, mut rx) = mpsc::unbounded_channel();
        let in_degree = Arc::new(Mutex::new(in_degree));
        let tasks = Arc::new(self.tasks.clone());
        let dependents = Arc::new(dependents);

        // Spawn tasks that have no pending dependencies.
        for (id, &degree) in in_degree.lock().unwrap().iter() {
            if degree == 0 {
                Self::spawn_task(
                    id.clone(),
                    tasks.clone(),
                    in_degree.clone(),
                    dependents.clone(),
                    tx.clone(),
                    self.storage.clone(),
                );
            }
        }

        // Wait until all tasks have finished.
        let total_tasks = tasks.len();
        let mut completed = 0;
        while let Some(completed_id) = rx.recv().await {
            info!("Workflow: task '{}' has completed", completed_id);
            completed += 1;
            if completed == total_tasks {
                break;
            }
        }
        Ok(())
    }

    fn spawn_task(
        task_id: String,
        tasks: Arc<HashMap<String, TaskWrapper>>,
        in_degree: Arc<Mutex<HashMap<String, usize>>>,
        dependents: Arc<HashMap<String, Vec<String>>>,
        tx: mpsc::UnboundedSender<String>,
        storage: Arc<dyn WorkflowStorage>,
    ) {
        tokio::spawn(async move {
            let wrapper = tasks.get(&task_id).unwrap().clone();
            let mut attempts = 0;
            let max_attempts = wrapper.retry_policy.max_retries + 1; // including the first attempt

            loop {
                attempts += 1;
                info!(
                    "Task '{}' starting (attempt {}/{})",
                    task_id, attempts, max_attempts
                );
                // Mark task as running.
                if let Err(e) = storage
                    .update_task_state(&task_id, "running", attempts)
                    .await
                {
                    eprintln!("Error updating state for task '{}': {}", task_id, e);
                }

                let result = wrapper.task.execute().await;
                match result {
                    Ok(_) => {
                        info!("Task '{}' completed successfully", task_id);
                        if let Err(e) = storage
                            .update_task_state(&task_id, "completed", attempts)
                            .await
                        {
                            eprintln!("Error updating state for task '{}': {}", task_id, e);
                        }
                        break;
                    }
                    Err(e) => {
                        eprintln!("Task '{}' failed on attempt {}: {}", task_id, attempts, e);
                        if attempts < max_attempts {
                            info!(
                                "Retrying task '{}' after {:?}",
                                task_id, wrapper.retry_policy.retry_delay
                            );
                            sleep(wrapper.retry_policy.retry_delay).await;
                        } else {
                            eprintln!("Task '{}' failed after {} attempts", task_id, attempts);
                            if let Err(e) = storage
                                .update_task_state(&task_id, "failed", attempts)
                                .await
                            {
                                eprintln!("Error updating state for task '{}': {}", task_id, e);
                            }
                            break;
                        }
                    }
                }
            }

            // Signal completion.
            tx.send(task_id.clone()).unwrap();

            // Update and spawn dependents if ready.
            if let Some(deps) = dependents.get(&task_id) {
                for dep_id in deps {
                    let mut degree_lock = in_degree.lock().unwrap();
                    if let Some(count) = degree_lock.get_mut(dep_id) {
                        *count -= 1;
                        if *count == 0 {
                            drop(degree_lock);
                            Self::spawn_task(
                                dep_id.clone(),
                                tasks.clone(),
                                in_degree.clone(),
                                dependents.clone(),
                                tx.clone(),
                                storage.clone(),
                            );
                        }
                    }
                }
            }
        });
    }
}

/// Represents a task along with its dependencies and retry policy.
#[derive(Clone)]
struct TaskWrapper {
    task: Arc<dyn Task>,
    dependencies: Vec<String>,
    retry_policy: RetryPolicy,
}
