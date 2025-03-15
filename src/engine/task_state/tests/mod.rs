#[cfg(test)]
mod tests {
    use super::super::task_state::TaskState;

    #[test]
    fn test_task_state_as_str() {
        assert_eq!(TaskState::Pending.as_str(), "pending");
        assert_eq!(TaskState::Running.as_str(), "running");
        assert_eq!(TaskState::Completed.as_str(), "completed");
        assert_eq!(TaskState::Failed.as_str(), "failed");
    }

    #[test]
    fn test_task_state_from_str() {
        assert_eq!(TaskState::from_str("pending"), Some(TaskState::Pending));
        assert_eq!(TaskState::from_str("running"), Some(TaskState::Running));
        assert_eq!(TaskState::from_str("completed"), Some(TaskState::Completed));
        assert_eq!(TaskState::from_str("failed"), Some(TaskState::Failed));
        assert_eq!(TaskState::from_str("invalid"), None);
    }

    #[test]
    fn test_task_state_display() {
        assert_eq!(format!("{}", TaskState::Pending), "pending");
        assert_eq!(format!("{}", TaskState::Running), "running");
        assert_eq!(format!("{}", TaskState::Completed), "completed");
        assert_eq!(format!("{}", TaskState::Failed), "failed");
    }
}
