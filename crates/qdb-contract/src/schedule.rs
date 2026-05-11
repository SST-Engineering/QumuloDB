//! CPM scheduling request and result types.

use serde::{Deserialize, Serialize};

use crate::calendar::KernelCalendar;

// ── Input types ───────────────────────────────────────────────────────────────

/// An activity as supplied to the schedule kernel.
/// Contains planning data only — no computed CPM results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityInput {
    pub id: String,
    pub name: String,

    /// Planned duration in working hours. Milestones have `duration_hours = 0.0`.
    pub duration_hours: f64,

    /// `"not_started"` | `"in_progress"` | `"completed"`
    #[serde(default = "default_status")]
    pub status: String,

    /// 0.0–100.0. Drives `remaining = duration × (1 − pct/100)`.
    #[serde(default)]
    pub percent_complete: f64,

    /// If `None`, the project default calendar is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calendar_id: Option<String>,

    /// `ASAP` | `ALAP` | `MSO` | `MFO` | `SNET` | `SNLT` | `FNET` | `FNLT`
    #[serde(default = "default_constraint")]
    pub constraint_type: String,

    /// ISO datetime. Required for MSO, MFO, SNET, SNLT, FNET, FNLT.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constraint_date: Option<String>,

    /// ISO datetime. Kernel emits `DEADLINE_MISSED` warning if EF > deadline.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deadline: Option<String>,

    /// ISO datetime. Set when status transitions to `in_progress`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_start: Option<String>,

    /// ISO datetime. Set when status transitions to `completed`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_finish: Option<String>,

    /// e.g. `"1.2.3"`. Used for WBS cost roll-ups in the EVM kernel.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wbs_code: Option<String>,

    /// Always `true` when `duration_hours == 0.0`.
    #[serde(default)]
    pub is_milestone: bool,
}

/// A logical dependency between two activities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyInput {
    pub predecessor_id: String,
    pub successor_id: String,

    /// `FS` | `SS` | `FF` | `SF`
    #[serde(default = "default_dep_type")]
    pub dep_type: String,

    /// Working hours of lag (positive) or lead (negative).
    #[serde(default)]
    pub lag_hours: f64,

    /// Set by the kernel — `true` when this dependency lies on the critical path.
    #[serde(default)]
    pub driving: bool,
}

/// Full input to the CPM scheduling kernel. Self-contained — no database references.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleRequest {
    pub project_id: String,

    /// ISO datetime. Anchor for the forward pass.
    pub project_start: String,

    pub activities: Vec<ActivityInput>,
    pub dependencies: Vec<DependencyInput>,

    /// ISO datetime. The "as-of" date for in-progress activities.
    /// Defaults to `project_start` when absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_date: Option<String>,

    /// ISO datetime. Hard deadline for the backward pass.
    /// When absent, `project_finish = max(early_finish)` of all activities.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_finish_constraint: Option<String>,

    /// All calendars referenced by `calendar_id` in activities.
    /// When empty, a standard Mon–Fri 08:00–17:00 calendar is used.
    #[serde(default)]
    pub calendars: Vec<KernelCalendar>,

    /// Calendar used for activities where `calendar_id` is `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_calendar_id: Option<String>,
}

// ── Result types ──────────────────────────────────────────────────────────────

/// A non-fatal issue detected during scheduling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleWarning {
    /// `CIRCULAR_DEPENDENCY` | `NEGATIVE_FLOAT` | `DEADLINE_MISSED`
    /// | `MISSING_PREDECESSOR` | `MISSING_SUCCESSOR`
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activity_id: Option<String>,
}

/// An activity as returned by the schedule kernel — planning data plus all CPM results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityResult {
    // ── Input fields echoed back ──────────────────────────────────────────────
    pub id: String,
    pub name: String,
    pub duration_hours: f64,
    pub status: String,
    pub percent_complete: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calendar_id: Option<String>,
    pub constraint_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constraint_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deadline: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_start: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_finish: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wbs_code: Option<String>,
    pub is_milestone: bool,

    // ── CPM forward pass ──────────────────────────────────────────────────────
    /// Earliest possible start. ISO datetime string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub early_start: Option<String>,

    /// Earliest possible finish. ISO datetime string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub early_finish: Option<String>,

    // ── CPM backward pass ─────────────────────────────────────────────────────
    /// Latest allowable start without delaying project finish.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub late_start: Option<String>,

    /// Latest allowable finish without delaying project finish.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub late_finish: Option<String>,

    // ── Float ─────────────────────────────────────────────────────────────────
    /// Working hours the activity can slip without delaying the project end.
    /// Negative value means the constraint is already violated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_float_hours: Option<f64>,

    /// Working hours available before the earliest successor is delayed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub free_float_hours: Option<f64>,

    /// Float available regardless of predecessor float consumption.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub independent_float_hours: Option<f64>,

    // ── Status flags ──────────────────────────────────────────────────────────
    /// `true` when `total_float_hours <= 0`.
    #[serde(default)]
    pub is_critical: bool,

    // ── Post-levelling dates ──────────────────────────────────────────────────
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled_start: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled_finish: Option<String>,
}

/// Full output of the CPM scheduling kernel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleResult {
    pub project_id: String,

    /// `false` only when the network contains circular dependencies.
    pub success: bool,

    /// ISO datetime. Echoed from request.
    pub project_start: String,

    /// ISO datetime. `max(early_finish)` or `project_finish_constraint`.
    pub project_finish: String,

    /// ISO datetime. Echoed from request (or `project_start` if absent).
    pub data_date: String,

    /// Working hours from `project_start` to `project_finish`.
    pub total_duration_hours: f64,

    pub activities: Vec<ActivityResult>,

    /// Ordered list of activity IDs on the critical path (start → finish).
    pub critical_path: Vec<String>,

    pub warnings: Vec<ScheduleWarning>,
    pub negative_float_count: usize,

    /// ISO datetime (UTC). When the kernel produced this result.
    pub computed_at: String,
}

// ── Defaults ──────────────────────────────────────────────────────────────────

fn default_status() -> String { "not_started".into() }
fn default_constraint() -> String { "ASAP".into() }
fn default_dep_type() -> String { "FS".into() }
