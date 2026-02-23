//! SQLite-backed persistence for Plan Mode.
//! Plans and tasks survive restarts — no more in-memory-only state.

use crate::plan_tool::{Plan, PlanStatus, PlanTask, TaskStatus, TaskType};
use rusqlite::{Connection, params};
use std::path::PathBuf;
use std::sync::Mutex;

/// SQLite-backed plan store that persists plans across restarts.
pub struct SqlitePlanStore {
    conn: Mutex<Connection>,
}

impl SqlitePlanStore {
    /// Open or create the plan database.
    pub fn open(path: &std::path::Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let conn = Connection::open(path).map_err(|e| format!("Failed to open plan DB: {e}"))?;

        // Enable WAL mode for better concurrent access
        conn.execute_batch("PRAGMA journal_mode=WAL;").ok();

        // Create tables
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS plans (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                description TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'Draft',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS plan_tasks (
                plan_id TEXT NOT NULL,
                task_idx INTEGER NOT NULL,
                title TEXT NOT NULL,
                description TEXT NOT NULL,
                task_type TEXT NOT NULL DEFAULT 'Other',
                status TEXT NOT NULL DEFAULT 'Pending',
                complexity INTEGER NOT NULL DEFAULT 1,
                dependencies TEXT NOT NULL DEFAULT '[]',
                created_at TEXT NOT NULL DEFAULT '',
                completed_at TEXT,
                result TEXT,
                PRIMARY KEY (plan_id, task_idx),
                FOREIGN KEY (plan_id) REFERENCES plans(id) ON DELETE CASCADE
            );",
        )
        .map_err(|e| format!("Failed to create plan tables: {e}"))?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Open the default plan database at `~/.bizclaw/plans.db`.
    pub fn open_default() -> Result<Self, String> {
        let path = Self::default_path();
        Self::open(&path)
    }

    /// Default database path.
    pub fn default_path() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".bizclaw").join("plans.db")
    }

    /// Load all plans from SQLite.
    pub fn load_all(&self) -> Vec<Plan> {
        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };

        let mut stmt = match conn.prepare(
            "SELECT id, title, description, status, created_at, updated_at FROM plans ORDER BY created_at ASC",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let plans: Vec<_> = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            })
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
            .unwrap_or_default();

        let mut result = Vec::new();
        for (id, title, description, status_str, created_at, updated_at) in plans {
            let status = parse_plan_status(&status_str);
            let tasks = self.load_tasks_for(&conn, &id);

            result.push(Plan {
                id,
                title,
                description,
                status,
                tasks,
                created_at,
                updated_at,
            });
        }

        result
    }

    /// Load tasks for a specific plan.
    fn load_tasks_for(&self, conn: &Connection, plan_id: &str) -> Vec<PlanTask> {
        let mut stmt = match conn.prepare(
            "SELECT task_idx, title, description, task_type, status, complexity, dependencies, created_at, completed_at, result
             FROM plan_tasks WHERE plan_id = ?1 ORDER BY task_idx ASC",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        stmt.query_map(params![plan_id], |row| {
            let id: usize = row.get::<_, i64>(0)? as usize;
            let title: String = row.get(1)?;
            let description: String = row.get(2)?;
            let task_type_str: String = row.get(3)?;
            let status_str: String = row.get(4)?;
            let complexity: u8 = row.get::<_, i32>(5)? as u8;
            let deps_str: String = row.get(6)?;
            let created_at: String = row.get(7)?;
            let completed_at: Option<String> = row.get(8)?;
            let result: Option<String> = row.get(9)?;

            let dependencies: Vec<usize> = serde_json::from_str(&deps_str).unwrap_or_default();

            Ok(PlanTask {
                id,
                title,
                description,
                task_type: parse_task_type(&task_type_str),
                status: parse_task_status(&status_str),
                complexity,
                dependencies,
                created_at,
                completed_at,
                result,
            })
        })
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default()
    }

    /// Save a single plan (upsert).
    pub fn save_plan(&self, plan: &Plan) {
        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => return,
        };

        conn.execute(
            "INSERT OR REPLACE INTO plans (id, title, description, status, created_at, updated_at) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                plan.id,
                plan.title,
                plan.description,
                format!("{}", plan.status),
                plan.created_at,
                plan.updated_at,
            ],
        )
        .ok();

        // Delete existing tasks and re-insert
        conn.execute(
            "DELETE FROM plan_tasks WHERE plan_id = ?1",
            params![plan.id],
        )
        .ok();

        for task in &plan.tasks {
            let deps_json =
                serde_json::to_string(&task.dependencies).unwrap_or_else(|_| "[]".to_string());
            conn.execute(
                "INSERT INTO plan_tasks (plan_id, task_idx, title, description, task_type, status, complexity, dependencies, created_at, completed_at, result)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    plan.id,
                    task.id as i64,
                    task.title,
                    task.description,
                    format!("{}", task.task_type),
                    format!("{}", task.status),
                    task.complexity as i32,
                    deps_json,
                    task.created_at,
                    task.completed_at,
                    task.result,
                ],
            )
            .ok();
        }
    }

    /// Save all plans (bulk sync from in-memory Vec).
    pub fn save_all(&self, plans: &[Plan]) {
        for plan in plans {
            self.save_plan(plan);
        }
    }

    /// Delete a plan by ID.
    pub fn delete_plan(&self, plan_id: &str) -> bool {
        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => return false,
        };
        conn.execute(
            "DELETE FROM plan_tasks WHERE plan_id = ?1",
            params![plan_id],
        )
        .ok();
        let deleted = conn
            .execute("DELETE FROM plans WHERE id = ?1", params![plan_id])
            .unwrap_or(0);
        deleted > 0
    }

    /// Get plan count.
    pub fn plan_count(&self) -> usize {
        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => return 0,
        };
        conn.query_row("SELECT COUNT(*) FROM plans", [], |r| r.get::<_, i64>(0))
            .unwrap_or(0) as usize
    }
}

// ---- Status parsing helpers ----

fn parse_plan_status(s: &str) -> PlanStatus {
    match s {
        "Draft" => PlanStatus::Draft,
        "Pending Approval" => PlanStatus::PendingApproval,
        "Approved" => PlanStatus::Approved,
        "In Progress" => PlanStatus::InProgress,
        "Completed" => PlanStatus::Completed,
        "Rejected" => PlanStatus::Rejected,
        _ => PlanStatus::Draft,
    }
}

fn parse_task_status(s: &str) -> TaskStatus {
    // Handle Display format (with emoji) and plain format
    if s.contains("Pending") {
        TaskStatus::Pending
    } else if s.contains("In Progress") {
        TaskStatus::InProgress
    } else if s.contains("Completed") {
        TaskStatus::Completed
    } else if s.contains("Skipped") {
        TaskStatus::Skipped
    } else if s.contains("Failed") {
        TaskStatus::Failed
    } else if s.contains("Blocked") {
        TaskStatus::Blocked
    } else {
        TaskStatus::Pending
    }
}

fn parse_task_type(s: &str) -> TaskType {
    if s.contains("Research") {
        TaskType::Research
    } else if s.contains("Edit") {
        TaskType::Edit
    } else if s.contains("Create") {
        TaskType::Create
    } else if s.contains("Delete") {
        TaskType::Delete
    } else if s.contains("Test") {
        TaskType::Test
    } else if s.contains("Refactor") {
        TaskType::Refactor
    } else if s.contains("Documentation") {
        TaskType::Documentation
    } else if s.contains("Configuration") {
        TaskType::Configuration
    } else if s.contains("Build") {
        TaskType::Build
    } else {
        TaskType::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqlite_plan_store_roundtrip() {
        let dir = std::env::temp_dir().join("bizclaw-test-planstore");
        std::fs::create_dir_all(&dir).ok();
        let db_path = dir.join("test-plans.db");
        let _ = std::fs::remove_file(&db_path); // Clean start

        let store = SqlitePlanStore::open(&db_path).unwrap();

        // Create and save a plan
        let mut plan = Plan::new("Test Plan", "A test plan for unit testing");
        plan.add_task("Task 1", "First task", TaskType::Research, 2, vec![]);
        plan.add_task("Task 2", "Second task", TaskType::Edit, 3, vec![0]);
        store.save_plan(&plan);

        // Load back
        let plans = store.load_all();
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].title, "Test Plan");
        assert_eq!(plans[0].tasks.len(), 2);
        assert_eq!(plans[0].tasks[1].dependencies, vec![0]);

        // Delete
        assert!(store.delete_plan(&plan.id));
        let plans = store.load_all();
        assert!(plans.is_empty());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_plan_persistence_across_loads() {
        let dir = std::env::temp_dir().join("bizclaw-test-planstore-persist");
        std::fs::create_dir_all(&dir).ok();
        let db_path = dir.join("persist-test.db");
        let _ = std::fs::remove_file(&db_path);

        // Save with one instance
        {
            let store = SqlitePlanStore::open(&db_path).unwrap();
            let plan = Plan::new("Persistent Plan", "Should survive close");
            store.save_plan(&plan);
        }

        // Load with new instance
        {
            let store = SqlitePlanStore::open(&db_path).unwrap();
            let plans = store.load_all();
            assert_eq!(plans.len(), 1);
            assert_eq!(plans[0].title, "Persistent Plan");
        }

        std::fs::remove_dir_all(&dir).ok();
    }
}
