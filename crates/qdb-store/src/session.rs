//! Per-project in-memory session — holds live Arrow RecordBatches.

use std::collections::HashMap;
use std::sync::Arc;

use arrow_array::RecordBatch;
use arrow_schema::Schema;

/// Live in-memory state for one project.
pub struct ProjectSession {
    pub project_id: String,
    /// Latest schedule snapshot record batch (schema: `schedule_snapshots_schema`).
    pub schedule_snapshot: Option<RecordBatch>,
    /// Latest activity record batches.
    pub activities: Vec<RecordBatch>,
    /// Latest EVM record batch.
    pub evm: Option<RecordBatch>,
    /// Latest simulation record batch.
    pub simulation: Option<RecordBatch>,
    /// Latest resource load record batches.
    pub resource_loads: Vec<RecordBatch>,
}

impl ProjectSession {
    pub fn new(project_id: impl Into<String>) -> Self {
        Self {
            project_id: project_id.into(),
            schedule_snapshot: None,
            activities: Vec::new(),
            evm: None,
            simulation: None,
            resource_loads: Vec::new(),
        }
    }
}

/// Thread-safe registry of active project sessions.
pub struct SessionRegistry {
    sessions: HashMap<String, ProjectSession>,
}

impl SessionRegistry {
    pub fn new() -> Self {
        Self { sessions: HashMap::new() }
    }

    pub fn get_or_create(&mut self, project_id: &str) -> &mut ProjectSession {
        self.sessions
            .entry(project_id.to_string())
            .or_insert_with(|| ProjectSession::new(project_id))
    }

    pub fn get(&self, project_id: &str) -> Option<&ProjectSession> {
        self.sessions.get(project_id)
    }

    pub fn get_mut(&mut self, project_id: &str) -> Option<&mut ProjectSession> {
        self.sessions.get_mut(project_id)
    }

    pub fn remove(&mut self, project_id: &str) {
        self.sessions.remove(project_id);
    }
}

impl Default for SessionRegistry {
    fn default() -> Self { Self::new() }
}
