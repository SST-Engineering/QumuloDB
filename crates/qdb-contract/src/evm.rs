//! Earned Value Management request and result types.

use serde::{Deserialize, Serialize};

/// Cumulative EVM values for one reporting period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EVMPeriodInput {
    /// ISO date string.
    pub period_start: String,
    /// ISO date string.
    pub period_end: String,

    /// Cumulative Planned Value (PV / BCWS) at `period_end`.
    /// The budgeted cost of work scheduled to be done by this date.
    pub planned_value: f64,

    /// Cumulative Earned Value (EV / BCWP) at `period_end`.
    /// The budgeted cost of work actually performed.
    pub earned_value: f64,

    /// Cumulative Actual Cost (AC / ACWP) at `period_end`.
    pub actual_cost: f64,
}

/// Input to the EVM calculation kernel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EVMRequest {
    pub project_id: String,

    /// Budget at Completion — total authorised budget for the project.
    pub bac: f64,

    /// ISO date string. The reporting cutoff date.
    pub data_date: String,

    /// Ordered list of cumulative EVM periods.
    /// The kernel uses the latest period at or before `data_date` for headline metrics.
    pub periods: Vec<EVMPeriodInput>,

    /// `"cpi"` | `"spi"` | `"remaining"` | `"manual"`
    #[serde(default = "default_eac_method")]
    pub eac_method: String,

    /// Only used when `eac_method = "manual"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manual_eac: Option<f64>,
}

/// Computed EVM metrics for one period in the S-curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EVMPeriodResult {
    pub period_start: String,
    pub period_end: String,
    pub cumulative_pv: f64,
    pub cumulative_ev: f64,
    pub cumulative_ac: f64,
    /// Delta PV for this period (cumulative − previous).
    pub period_pv: f64,
    pub period_ev: f64,
    pub period_ac: f64,
    /// EV / AC for this period only. `None` if AC = 0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period_cpi: Option<f64>,
    /// EV / PV for this period only. `None` if PV = 0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period_spi: Option<f64>,
}

/// Full output of the EVM calculation kernel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EVMResult {
    pub project_id: String,
    pub data_date: String,
    pub bac: f64,

    // ── Headline figures at data_date ─────────────────────────────────────────
    /// Planned Value (BCWS) at data_date.
    pub pv: f64,
    /// Earned Value (BCWP) at data_date.
    pub ev: f64,
    /// Actual Cost (ACWP) at data_date.
    pub ac: f64,
    /// Cost Variance = EV − AC. Negative = over budget.
    pub cv: f64,
    /// Schedule Variance = EV − PV. Negative = behind schedule.
    pub sv: f64,
    /// CV as % of EV.
    pub cv_pct: f64,
    /// SV as % of PV.
    pub sv_pct: f64,
    /// Cost Performance Index = EV / AC. < 1.0 = over budget.
    pub cpi: f64,
    /// Schedule Performance Index = EV / PV. < 1.0 = behind schedule.
    pub spi: f64,

    // ── Forecast ──────────────────────────────────────────────────────────────
    /// Estimate at Completion — predicted total cost.
    pub eac: f64,
    /// Estimate to Complete = EAC − AC.
    pub etc: f64,
    /// Variance at Completion = BAC − EAC. Negative = projected over-run.
    pub vac: f64,
    /// To-Complete Performance Index = (BAC − EV) / (BAC − AC).
    pub tcpi: f64,
    /// EV / BAC × 100.
    pub percent_complete: f64,
    /// AC / BAC × 100.
    pub percent_spent: f64,

    pub eac_method: String,

    /// S-curve data — one entry per input period.
    #[serde(default)]
    pub periods: Vec<EVMPeriodResult>,

    /// ISO datetime (UTC).
    pub computed_at: String,
}

fn default_eac_method() -> String { "cpi".into() }
