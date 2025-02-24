use crate::storage::WorkflowStorage;
use async_trait::async_trait;
use sqlx::{sqlite::SqlitePoolOptions, Pool, Row, Sqlite};
use std::error::Error;

pub struct SqliteStorage {
    pub pool: Pool<Sqlite>,
}

impl SqliteStorage {
    pub async fn new(database_url: &str) -> Result<Self, Box<dyn Error>> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }
}

#[async_trait]
impl WorkflowStorage for SqliteStorage {
    async fn init(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                workflow_id TEXT,
                state TEXT,
                attempts INTEGER,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn create_task_record(
        &self,
        workflow_id: &str,
        task_id: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO tasks (id, workflow_id, state, attempts)
            VALUES (?, ?, 'pending', 0)
            "#,
        )
        .bind(task_id)
        .bind(workflow_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn update_task_state(
        &self,
        task_id: &str,
        state: &str,
        attempts: usize,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        sqlx::query(
            r#"
            UPDATE tasks
            SET state = ?, attempts = ?, updated_at = CURRENT_TIMESTAMP
            WHERE id = ?
            "#,
        )
        .bind(state)
        .bind(attempts as i64)
        .bind(task_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_workflow_status(
        &self,
        workflow_id: &str,
    ) -> Result<Vec<(String, String, usize)>, Box<dyn Error + Send + Sync>> {
        let rows = sqlx::query(
            r#"
          SELECT id, state, attempts
          FROM tasks
          WHERE workflow_id = ?
          "#,
        )
        .bind(workflow_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| {
                let id: String = row.get(0);
                let state: String = row.get(1);
                let attempts: i64 = row.get(2);
                (id, state, attempts as usize)
            })
            .collect())
    }
}
