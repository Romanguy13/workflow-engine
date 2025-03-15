//! # Workflow Engine
//!
//! A multi-threaded, asynchronous workflow engine for Rust that allows
//! defining and executing workflows with complex task dependencies.
//!
//! ## Features
//!
//! - Define workflows with tasks and dependencies
//! - Asynchronous task execution
//! - Parallel task execution
//! - Task retries on failure
//! - Customizable task retry policies
//! - Workflow options for controlling execution
//! - Workflow state persistence
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! workflow_engine = "0.1"
//! ```
//!
//! ## Example
//!
//! ```rust
//! use workflow_engine::storage::implementations::MemoryStorage;
//! use workflow_engine::{Workflow, Task, TaskState};
//! use async_trait::async_trait;
//! use std::error::Error;
//! use std::sync::Arc;
//!
//! // Define a custom task
//! struct MyTask;
//!
//! #[async_trait::async_trait]
//! impl Task for MyTask {
//!     async fn execute(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!         println!("Executing task...");
//!         Ok(())
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     // Create a new workflow
//!     let workflow = Workflow::new("my_workflow", Arc::new(MemoryStorage::new()))
//!         .add_task("task1", MyTask, vec![], None)  // Add a task with no dependencies
//!         .add_task("task2", MyTask, vec!["task1".into()], None);  // Add a task with a dependency
//!
//!     // Execute the workflow
//!     let result = workflow.execute().await;
//!     match result {
//!         Ok(_) => println!("Workflow completed successfully"),
//!         Err(e) => eprintln!("Workflow failed: {}", e),
//!     }
//! }
//! ```
//!
//! ## License
//!
//! Licensed under the MIT license. See the [LICENSE](LICENSE) file for details.

pub mod engine;
pub mod storage;
pub mod task;

pub use engine::{TaskState, Workflow};
pub use storage::WorkflowStorage;
pub use task::{RetryPolicy, Task};
