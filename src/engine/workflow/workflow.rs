use log::{debug, info};
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

use super::super::dependency::DependencyGraph;
use super::super::execution::{ExecutionEnvironment, WorkflowSharedState};
use super::super::executor::TaskExecutor;
use super::super::task_state::TaskState;
use super::super::task_wrapper::TaskWrapper;
use super::super::validation;
use crate::storage::WorkflowStorage;
use crate::task::{RetryPolicy, Task};

pub struct Workflow {
    /// Map of task IDs to task wrappers
    tasks: HashMap<String, TaskWrapper>,
    /// Unique identifier for the workflow
    workflow_id: String,
    /// Storage backend for persisting workflow state
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
        debug!(
            "Executing workflow '{}' with {} tasks",
            self.workflow_id,
            self.tasks.len()
        );

        if let Err(e) = validation::validate_workflow_tasks(&self.tasks) {
            return Err(e);
        }

        self.initialize_storage_records().await?;

        let dependency_graph = self.build_dependency_graph();

        let execution_env = self.setup_execution_environment(dependency_graph);

        let executor = TaskExecutor::new(execution_env.shared_state.clone());
        self.spawn_initial_tasks(&execution_env.shared_state, &executor);

        self.wait_for_completion(execution_env).await;

        debug!("Workflow '{}' completed", self.workflow_id);
        Ok(())
    }

    async fn initialize_storage_records(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        debug!(
            "Initializing storage records for workflow '{}'",
            self.workflow_id
        );

        for task_id in self.tasks.keys() {
            self.storage
                .create_task_record(&self.workflow_id, task_id)
                .await?
        }

        Ok(())
    }

    fn build_dependency_graph(&self) -> DependencyGraph {
        debug!(
            "Building dependency graph for workflow '{}'",
            self.workflow_id
        );

        // Calculate in-degree (number of dependencies) for each task
        let in_degree: HashMap<String, usize> = self
            .tasks
            .iter()
            .map(|(id, wrapper)| (id.clone(), wrapper.dependencies.len()))
            .collect();

        // Build reverse mapping: which tasks depend on each task
        let mut dependents: HashMap<String, Vec<String>> = HashMap::new();
        for (id, wrapper) in &self.tasks {
            for dep in &wrapper.dependencies {
                dependents.entry(dep.clone()).or_default().push(id.clone());
            }
        }

        DependencyGraph {
            in_degree,
            dependents,
        }
    }

    fn setup_execution_environment(
        &self,
        dependency_graph: DependencyGraph,
    ) -> ExecutionEnvironment {
        debug!(
            "Setting up execution environment for workflow '{}'",
            self.workflow_id
        );

        let (tx, rx) = mpsc::unbounded_channel();
        let task_count = self.tasks.len();

        let shared_state = WorkflowSharedState {
            tasks: Arc::new(self.tasks.clone()),
            in_degree: Arc::new(Mutex::new(dependency_graph.in_degree)),
            dependents: Arc::new(dependency_graph.dependents),
            completion_sender: tx,
            storage: self.storage.clone(),
        };

        ExecutionEnvironment {
            task_count,
            shared_state,
            completion_channel: rx,
        }
    }

    fn spawn_initial_tasks(&self, shared_state: &WorkflowSharedState, executor: &TaskExecutor) {
        debug!("Spawning initial tasks for workflow '{}'", self.workflow_id);

        let mut tasks_to_spawn = Vec::new();

        // First collect all tasks with zero dependencies
        {
            let in_degree_lock = shared_state.in_degree.lock().unwrap();
            for (id, &degree) in in_degree_lock.iter() {
                if degree == 0 {
                    tasks_to_spawn.push(id.clone());
                }
            }
        } // Lock is released here

        info!("Spawning {} initial tasks", tasks_to_spawn.len());

        // Then spawn them all using the executor
        for task_id in tasks_to_spawn {
            executor.spawn_task(task_id);
        }
    }

    async fn wait_for_completion(&self, execution_env: ExecutionEnvironment) {
        let ExecutionEnvironment {
            task_count,
            completion_channel: mut rx,
            ..
        } = execution_env;

        debug!("Waiting for {} tasks to complete", task_count);

        let mut completed = 0;
        while let Some(completed_id) = rx.recv().await {
            info!("Workflow: task '{}' has completed", completed_id);
            completed += 1;

            debug!("Progress: {}/{} tasks completed", completed, task_count);

            if completed == task_count {
                break;
            }
        }

        debug!("All tasks completed for workflow '{}'", self.workflow_id);
    }

    pub async fn get_status(
        &self,
    ) -> Result<HashMap<String, TaskState>, Box<dyn Error + Send + Sync>> {
        let status = self.storage.get_workflow_status(&self.workflow_id).await?;

        let mut result = HashMap::new();
        for (id, state, _) in status {
            result.insert(id, state);
        }

        Ok(result)
    }
}
