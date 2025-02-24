# Rust Workflow Engine

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

A multi-threaded, asynchronous engine for defining, scheduling, and executing complex workflows. Built with Rust and powered by Tokio, it allows you to create dynamic, dependency-driven workflows with built-in support for retries, persistence, and real-time monitoring.

## Table of Contents

- [Features](#features)
- [Getting Started](#getting-started)
- [Usage](#usage)
- [Architecture](#architecture)
- [Development](#development)
- [License](#license)

## Features

- **Dynamic Workflow Definition:** Define workflows and tasks at runtime.
- **Dependency Management:** Configure tasks with dependencies for sequential or parallel execution.
- **Asynchronous Execution:** Built with Tokio for high-performance, concurrent task execution.
- **Retry Policies:** Automatic task retries on failure with customizable delay.
- **State Persistence:** Track task and workflow status with a pluggable storage backend (SQLite, PostgreSQL, etc.).
- **Extensible API:** REST and gRPC endpoints for external control and monitoring.
- **CLI and Dashboard (Planned):** Tools to manage workflows and visualize execution progress.

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (1.56+ recommended)
- [SQLite](https://www.sqlite.org/index.html) (if using the SQLite storage backend)
- Git

### Installation

1. **Clone the repository:**

   ```sh
   git clone https://github.com/yourname/rust-workflow-engine.git
   cd rust-workflow-engine
   ```

2. **Install sqlx-cli globally:**

   ```sh
   cargo install sqlx-cli
   ```

3. **Run the setup script:**

   ```sh
    ./scripts/setup.sh
   ```

4. **Run the engine:**

   ```sh
   cargo run
   ```

## Usage

### Defining a Workflow

Below is a simple example of how to define and run a workflow with two tasks:

**Rust Code:**

```rust
use workflow_engine::engine::Workflow;
use workflow_engine::task::ExampleTask;
use workflow_engine::task::RetryPolicy;
use workflow_engine::storage::sqlite_storage::SqliteStorage;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // Initialize storage backend (SQLite example)
    let storage = Arc::new(SqliteStorage::new("sqlite:workflow.db").await.unwrap());
    storage.init().await.unwrap();

    // Build the workflow with a unique workflow ID.
    let workflow = Workflow::new("example_workflow", storage)
        .add_task(
          "task1",
          ExampleTask { name: "Task 1".into(), fail_once: false },
          vec![],
          None
        )
        .add_task(
          "task2",
          ExampleTask { name: "Task 2".into(), fail_once: true },
          vec!["task1".into()],
          Some(RetryPolicy { max_retries: 2, retry_delay: std::time::Duration::from_secs(2) })
        );

    // Execute the workflow.
    workflow.execute().await.unwrap();
}
```

### API and CLI

The project will eventually include a REST API and CLI tool ('workflowctl') for managing workflows externally. Stay tuned for updates in the documentation.

## Architecture

The engine is built around a modular design:

- **Engine Core:** Manages task scheduling, dependency resolution, and concurrent execution.
- **Task Abstraction:** All tasks implement a common 'Task' trait, allowing for custom user-defined behavior.
- **Storage Layer:** A pluggable persistence backend records task state, execution history, and provides real-time status updates.
- **API & CLI:** Interfaces for interacting with the engine programmatically and via command-line.

## Development

### Code Style

- Use [rustfmt](https://github.com/rust-lang/rustfmt) to format your code:

  ```sh
  cargo fmt
  ```

- Write tests for new features and bug fixes.
- Follow Rust's idiomatic patterns and naming conventions.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## Acknowledgments

- [Tokio](https://tokio.rs/) for the asynchronous runtime.
- [sqlx](https://github.com/launchbadge/sqlx) for the SQL toolkit.
- All contributors who helped shape this project.

Happy Coding! 🚀
