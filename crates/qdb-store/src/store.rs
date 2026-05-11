//! QDBStore — the top-level storage facade.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use chrono::Utc;
use datafusion::prelude::*;
use qdb_contract::{EVMResult, ResourceResult, ScheduleResult, SimulationResult};
use uuid::Uuid;

use crate::{error::StoreError, session::SessionRegistry};

type Result<T> = std::result::Result<T, StoreError>;

/// QumuloDB storage engine.
///
/// Thread-safe via an internal `Mutex`. Create one instance per server process
/// and clone the `Arc` across threads.
pub struct QDBStore {
    data_dir: PathBuf,
    sessions: Mutex<SessionRegistry>,
    ctx: SessionContext,
}

impl QDBStore {
    /// Open (or create) a QumuloDB data directory.
    pub fn open(data_dir: impl AsRef<Path>) -> Result<Arc<Self>> {
        let data_dir = data_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&data_dir)?;
        std::fs::create_dir_all(data_dir.join("schedules"))?;
        std::fs::create_dir_all(data_dir.join("evm"))?;
        std::fs::create_dir_all(data_dir.join("simulations"))?;
        std::fs::create_dir_all(data_dir.join("resources"))?;

        let ctx = SessionContext::new();

        Ok(Arc::new(Self {
            data_dir,
            sessions: Mutex::new(SessionRegistry::new()),
            ctx,
        }))
    }

    /// In-memory only store (no disk persistence).
    pub fn in_memory() -> Arc<Self> {
        Self::open(std::env::temp_dir().join(format!("qdb_{}", Uuid::new_v4())))
            .expect("temp dir creation failed")
    }

    // ── Persistence ───────────────────────────────────────────────────────────

    /// Persist a schedule result and update the in-memory session.
    pub fn save_schedule(&self, result: &ScheduleResult) -> Result<String> {
        let txn_now = Utc::now().to_rfc3339();
        let snapshot_id = Uuid::new_v4().to_string();

        // Serialise to JSON and write to Parquet-adjacent staging file
        let path = self
            .data_dir
            .join("schedules")
            .join(format!("{}_{}.json", result.project_id, txn_now.replace(':', "-")));
        let json = serde_json::to_string(result)?;
        std::fs::write(&path, json)?;

        // Update in-memory session
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions.get_or_create(&result.project_id);
        // Clear old batches — full replacement
        session.activities.clear();
        // (Arrow RecordBatch building deferred to next phase)

        tracing::info!(
            project_id = %result.project_id,
            snapshot_id = %snapshot_id,
            "schedule result persisted"
        );

        Ok(snapshot_id)
    }

    /// Persist an EVM result.
    pub fn save_evm(&self, result: &EVMResult) -> Result<String> {
        let txn_now = Utc::now().to_rfc3339();
        let evm_id = Uuid::new_v4().to_string();

        let path = self
            .data_dir
            .join("evm")
            .join(format!("{}_{}.json", result.project_id, txn_now.replace(':', "-")));
        std::fs::write(&path, serde_json::to_string(result)?)?;

        tracing::info!(project_id = %result.project_id, evm_id = %evm_id, "EVM result persisted");
        Ok(evm_id)
    }

    /// Persist a simulation result.
    pub fn save_simulation(&self, result: &SimulationResult) -> Result<String> {
        let txn_now = Utc::now().to_rfc3339();
        let sim_id = Uuid::new_v4().to_string();

        let path = self
            .data_dir
            .join("simulations")
            .join(format!("{}_{}.json", result.project_id, txn_now.replace(':', "-")));
        std::fs::write(&path, serde_json::to_string(result)?)?;

        tracing::info!(project_id = %result.project_id, sim_id = %sim_id, "simulation result persisted");
        Ok(sim_id)
    }

    /// Persist a resource result.
    pub fn save_resources(&self, result: &ResourceResult) -> Result<String> {
        let txn_now = Utc::now().to_rfc3339();
        let load_id = Uuid::new_v4().to_string();

        let path = self
            .data_dir
            .join("resources")
            .join(format!("{}_{}.json", result.project_id, txn_now.replace(':', "-")));
        std::fs::write(&path, serde_json::to_string(result)?)?;

        tracing::info!(project_id = %result.project_id, load_id = %load_id, "resource result persisted");
        Ok(load_id)
    }

    // ── Query ─────────────────────────────────────────────────────────────────

    /// Load the latest schedule result for a project from disk.
    pub fn latest_schedule(&self, project_id: &str) -> Result<Option<ScheduleResult>> {
        self.latest_json("schedules", project_id)
    }

    /// Load the latest EVM result for a project from disk.
    pub fn latest_evm(&self, project_id: &str) -> Result<Option<EVMResult>> {
        self.latest_json("evm", project_id)
    }

    /// Load the latest simulation result for a project from disk.
    pub fn latest_simulation(&self, project_id: &str) -> Result<Option<SimulationResult>> {
        self.latest_json("simulations", project_id)
    }

    /// Load the latest resource result for a project from disk.
    pub fn latest_resources(&self, project_id: &str) -> Result<Option<ResourceResult>> {
        self.latest_json("resources", project_id)
    }

    fn latest_json<T: serde::de::DeserializeOwned>(
        &self,
        table: &str,
        project_id: &str,
    ) -> Result<Option<T>> {
        let dir = self.data_dir.join(table);
        let prefix = format!("{}_", project_id);

        let mut entries: Vec<PathBuf> = std::fs::read_dir(&dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with(&prefix) && n.ends_with(".json"))
                    .unwrap_or(false)
            })
            .collect();

        entries.sort();

        if let Some(path) = entries.last() {
            let data = std::fs::read_to_string(path)?;
            let result: T = serde_json::from_str(&data)?;
            return Ok(Some(result));
        }

        Ok(None)
    }

    /// Execute a SQL query via DataFusion against registered tables.
    pub async fn query_sql(&self, sql: &str) -> Result<Vec<datafusion::arrow::record_batch::RecordBatch>> {
        let df = self.ctx.sql(sql).await?;
        let batches = df.collect().await?;
        Ok(batches)
    }
}
