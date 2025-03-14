#[derive(Debug, Clone)]
pub struct WorkflowOptions {
    /// Maximum number of concurrent tasks
    pub max_concurrency: Option<usize>,
}

impl Default for WorkflowOptions {
    fn default() -> Self {
        Self {
            max_concurrency: None,
        }
    }
}

impl WorkflowOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_max_concurrency(mut self, value: usize) -> Self {
        self.max_concurrency = Some(value);
        self
    }
}
