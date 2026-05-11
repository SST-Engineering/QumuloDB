//! Resource scheduling and levelling request and result types.

use serde::{Deserialize, Serialize};

use crate::calendar::KernelCalendar;

// ── Input types ───────────────────────────────────────────────────────────────

/// A resource (person, equipment, material) available to the project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceInput {
    pub resource_id: String,
    pub name: String,

    /// `"labour"` | `"equipment"` | `"material"`
    #[serde(default = "default_resource_type")]
    pub resource_type: String,

    /// Max units available at any point in time (1.0 = 100%).
    #[serde(default = "default_max_units")]
    pub max_units: f64,

    /// Cost per working hour.
    #[serde(default)]
    pub cost_per_hour: f64,

    /// Calendar ID. Falls back to project default when absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calendar_id: Option<String>,
}

/// An assignment — a resource allocated to an activity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignmentInput {
    pub assignment_id: String,
    pub activity_id: String,
    pub resource_id: String,

    /// Fraction of max_units allocated (0.0–1.0). 1.0 = 100%.
    #[serde(default = "default_units")]
    pub units: f64,

    /// `"fixed_units"` | `"fixed_work"` | `"fixed_duration"`
    #[serde(default = "default_work_type")]
    pub work_type: String,
}

/// Full input to the resource scheduling / levelling kernel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequest {
    pub project_id: String,

    pub resources: Vec<ResourceInput>,
    pub assignments: Vec<AssignmentInput>,

    /// Activity results from a prior CPM run — drives resource loading dates.
    /// The kernel reads `early_start`/`early_finish` (or `scheduled_start`/
    /// `scheduled_finish` if present) from each activity.
    pub activities: Vec<crate::schedule::ActivityResult>,

    /// ISO datetime. As-of date for in-progress loading.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_date: Option<String>,

    /// ISO datetime. Start of the report window.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report_start: Option<String>,

    /// ISO datetime. End of the report window.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report_end: Option<String>,

    /// Granularity of the load histogram: `"daily"` | `"weekly"` | `"monthly"`.
    #[serde(default = "default_period_granularity")]
    pub period_granularity: String,

    /// Whether to run the levelling algorithm.
    #[serde(default = "default_true")]
    pub level_resources: bool,

    /// All calendars referenced by resources or activities.
    #[serde(default)]
    pub calendars: Vec<KernelCalendar>,

    /// Calendar used when a resource has no explicit `calendar_id`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_calendar_id: Option<String>,
}

// ── Result types ──────────────────────────────────────────────────────────────

/// Load for one resource in one time period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodLoadResult {
    /// ISO datetime — period start.
    pub period_start: String,
    /// ISO datetime — period end.
    pub period_end: String,
    /// Total assigned working hours in this period.
    pub assigned_hours: f64,
    /// Available hours (capacity) in this period.
    pub available_hours: f64,
    /// assigned_hours / available_hours. > 1.0 = over-allocated.
    pub utilisation: f64,
    /// Hours by which assigned exceeds available (0 if not over-allocated).
    pub overallocation_hours: f64,
}

/// Full load profile for one resource across all periods.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLoadResult {
    pub resource_id: String,
    pub name: String,
    pub resource_type: String,
    pub max_units: f64,
    /// Total assigned hours across the entire report window.
    pub total_assigned_hours: f64,
    /// Total available hours across the entire report window.
    pub total_available_hours: f64,
    /// Peak utilisation fraction across all periods.
    pub peak_utilisation: f64,
    /// `true` when any period has `utilisation > 1.0`.
    pub is_over_allocated: bool,
    /// Period-by-period breakdown.
    pub periods: Vec<PeriodLoadResult>,
}

/// An adjustment made by the levelling algorithm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevellingAdjustment {
    pub activity_id: String,
    /// Hours the activity was delayed.
    pub delay_hours: f64,
    /// New scheduled start after levelling. ISO datetime.
    pub new_start: String,
    /// New scheduled finish after levelling. ISO datetime.
    pub new_finish: String,
    /// Resource ID that caused the delay.
    pub reason_resource_id: String,
}

/// Full output of the resource scheduling / levelling kernel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceResult {
    pub project_id: String,

    /// Load profile per resource.
    pub resources: Vec<ResourceLoadResult>,

    /// Adjustments applied when `level_resources = true`.
    #[serde(default)]
    pub levelling_adjustments: Vec<LevellingAdjustment>,

    /// `true` if any resource remains over-allocated after levelling.
    pub has_overallocation: bool,

    /// Total cost across all assignments (hours × cost_per_hour).
    pub total_cost: f64,

    /// ISO datetime (UTC).
    pub computed_at: String,
}

fn default_resource_type() -> String { "labour".into() }
fn default_max_units() -> f64 { 1.0 }
fn default_units() -> f64 { 1.0 }
fn default_work_type() -> String { "fixed_units".into() }
fn default_period_granularity() -> String { "daily".into() }
fn default_true() -> bool { true }
