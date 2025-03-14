use std::collections::HashMap;

pub(crate) struct DependencyGraph {
    /// Number of dependencies for each task
    pub in_degree: HashMap<String, usize>,
    /// Tasks that depend on each task
    pub dependents: HashMap<String, Vec<String>>,
}
