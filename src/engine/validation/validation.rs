use std::collections::{HashMap, HashSet};
use std::error::Error;

use super::super::task_wrapper::TaskWrapper;

pub fn validate_workflow_tasks(
    tasks: &HashMap<String, TaskWrapper>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    // Check for missing dependencies
    let all_task_ids: HashSet<String> = tasks.keys().cloned().collect();

    for (task_id, wrapper) in tasks {
        for dep in &wrapper.dependencies {
            if !all_task_ids.contains(dep) {
                return Err(
                    format!("Task '{}' depends on non-existent task '{}'", task_id, dep).into(),
                );
            }
        }
    }

    // Check for cyclic dependencies using DFS
    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();

    for task_id in tasks.keys() {
        if has_cycle(tasks, task_id, &mut visited, &mut rec_stack) {
            return Err(format!(
                "Cyclic dependency detected starting from task '{}'",
                task_id
            )
            .into());
        }
    }

    Ok(())
}

/// DFS helper to detect cycles in dependency graph
fn has_cycle(
    tasks: &HashMap<String, TaskWrapper>,
    task_id: &str,
    visited: &mut HashSet<String>,
    rec_stack: &mut HashSet<String>,
) -> bool {
    if !visited.contains(task_id) {
        visited.insert(task_id.to_string());
        rec_stack.insert(task_id.to_string());

        if let Some(wrapper) = tasks.get(task_id) {
            for dep in &wrapper.dependencies {
                if !visited.contains(dep) && has_cycle(tasks, dep, visited, rec_stack) {
                    return true;
                } else if rec_stack.contains(dep) {
                    return true;
                }
            }
        }
    }

    rec_stack.remove(task_id);
    false
}
