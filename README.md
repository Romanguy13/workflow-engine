# Rust Workflow Engine

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

A multi-threaded, asynchronous engine for defining, scheduling, and executing complex workflows. Built with Rust and powered by Tokio, it allows you to create dynamic, dependency-driven workflows with built-in support for retries, persistence, and real-time monitoring.

## Table of Contents

- [Features](#features)
- [Getting Started](#getting-started)
- [Usage](#usage)
- [Examples](#examples)
- [Project Structure](#project-structure)
- [Extending the Engine](#extending-the-engine)

## Features

- **Dynamic Workflow Definition:** Define workflows and tasks at runtime.
- **Dependency Management:** Configure tasks with dependencies for sequential or parallel execution.
- **Asynchronous Execution:** Built with Tokio for high-performance, concurrent task execution.
- **Retry Policies:** Automatic task retries on failure with customizable delay.
- **State Persistence:** Track task and workflow status with a pluggable storage backend (SQLite, PostgreSQL, etc.).

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (1.56+ recommended)
- [SQLite](https://www.sqlite.org/index.html) (If using the SQLite storage backend)

### Installation

**As a Dependency:**

```sh
[dependencies]
workflow-engine = { git = "https://github.com/romanguy13/workflow-engine" }
```

**From Source:**

```sh
git clone https://github.com/romanguy13/workflow-engine.git
cd rust-workflow-engine
cargo build
```

## Usage

### Defining a Workflow

Below is a simple example of how to define and run a workflow:

**Rust Code:**

```rust
use std::sync::Arc;
use workflow_engine::{Workflow, RetryPolicy};
use workflow_engine::storage::sqlite_storage::SqliteStorage;
use workflow_engine::task::Task;

// Define a custom task
struct SimpleTask {
    name: String,
}

#[async_trait::async_trait]
impl Task for SimpleTask {
    async fn execute(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("Executing task: {}", self.name);
        // Do some work here
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize storage
    let storage = Arc::new(SqliteStorage::new("sqlite:workflow.db").await?);
    storage.init().await?;

    // Build workflow
    let workflow = Workflow::new("simple_workflow", storage)
        .add_task("task1", SimpleTask { name: "First Task".into() }, vec![], None)
        .add_task(
            "task2",
            SimpleTask { name: "Second Task".into() },
            vec!["task1".into()],
            Some(RetryPolicy::default().with_max_retries(2))
        );

    // Execute workflow
    workflow.execute().await?;

    Ok(())
}
```

## Examples

### Running Examples

Several examples are provided in the `/examples` directory. To run an example:

```sh
# Run the advanced workflow example with options
cargo run --example with_options
```

## Extending the Engine

### Custom Tasks

Implement the `Task` trait to create custom tasks:

```rust
#[async_trait::async_trait]
impl Task for MyCustomTask {
    async fn execute(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Implementation goes here
        Ok(())
    }
}
```

### Custom Storage

Implement the `WorkflowStorage` trait to create a custom storage backend. An SQLite storage implementation is provided in the `storage` module for reference.

```rust
#[async_trait::async_trait]
impl WorkflowStorage for MyCustomStorage {
    async fn init(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Initialize storage
    }

    async fn create_task_record(&self, workflow_id: &str, task_id: &str)
        -> Result<(), Box<dyn Error + Send + Sync>> {
        // Create task record
    }

    // Additional required methods...
}
```

## Development

### Code Style

- Use [rustfmt](https://github.com/rust-lang/rustfmt) to format your code:

  ```sh
  cargo fmt
  ```

- Follow Rust's idiomatic patterns and naming conventions.

### Testing

- Run tests with:

  ```sh
  cargo test
  ```

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## Acknowledgments

- [Tokio](https://tokio.rs/) for the asynchronous runtime.
- All contributors who helped shape this project.

Happy Coding! 🚀
