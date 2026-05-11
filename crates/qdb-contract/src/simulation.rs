//! Monte Carlo simulation request and result types.

use serde::{Deserialize, Serialize};

/// A single quantified risk for the simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskInput {
    pub risk_id: String,
    pub name: String,

    /// 0.0–1.0 probability the risk materialises.
    pub probability: f64,

    /// `"triangular"` | `"pert"` | `"uniform"` | `"fixed"`
    #[serde(default = "default_distribution")]
    pub distribution: String,

    /// Minimum cost impact (or fixed value when distribution = "fixed").
    pub min_cost: f64,
    /// Most-likely cost impact.
    pub ml_cost: f64,
    /// Maximum cost impact.
    pub max_cost: f64,

    /// Minimum schedule impact in working hours.
    pub min_schedule_hours: f64,
    /// Most-likely schedule impact in working hours.
    pub ml_schedule_hours: f64,
    /// Maximum schedule impact in working hours.
    pub max_schedule_hours: f64,

    /// Activity IDs affected by this risk.
    #[serde(default)]
    pub affected_activity_ids: Vec<String>,
}

/// Input to the Monte Carlo simulation kernel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationRequest {
    pub project_id: String,

    /// Base (deterministic) cost from EVM or estimate.
    pub base_cost: f64,

    /// Base schedule duration in working hours.
    pub base_duration_hours: f64,

    /// ISO datetime. Project start anchor for schedule simulations.
    pub project_start: String,

    pub risks: Vec<RiskInput>,

    /// Number of iterations to run.
    #[serde(default = "default_iterations")]
    pub iterations: u32,

    /// Random seed for reproducible results. `None` = non-deterministic.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,

    /// Confidence levels to compute (e.g. `[0.5, 0.8, 0.9]`).
    /// Defaults to `[0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9]`.
    #[serde(default = "default_confidence_levels")]
    pub confidence_levels: Vec<f64>,
}

/// One bin in the output histogram.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramBin {
    pub bin_start: f64,
    pub bin_end: f64,
    pub count: u32,
    /// count / iterations — probability mass of this bin.
    pub frequency: f64,
    /// Cumulative probability up to and including this bin.
    pub cumulative: f64,
}

/// The contribution of a single risk to overall cost/schedule uncertainty.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskContributionResult {
    pub risk_id: String,
    pub name: String,
    /// Mean cost impact across all iterations.
    pub mean_cost_impact: f64,
    /// Mean schedule impact (hours) across all iterations.
    pub mean_schedule_impact_hours: f64,
    /// Pearson correlation with total cost outcome.
    pub cost_correlation: f64,
    /// Pearson correlation with total duration outcome.
    pub schedule_correlation: f64,
    /// Number of iterations in which this risk materialised.
    pub materialised_count: u32,
}

/// One entry in the tornado chart — sorted descending by `|cost_correlation|`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TornadoEntry {
    pub risk_id: String,
    pub name: String,
    pub cost_correlation: f64,
    pub schedule_correlation: f64,
}

/// Full output of the Monte Carlo simulation kernel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    pub project_id: String,
    pub iterations: u32,

    // ── Cost statistics ───────────────────────────────────────────────────────
    pub mean_cost: f64,
    pub std_dev_cost: f64,
    pub min_cost: f64,
    pub max_cost: f64,

    // ── Schedule statistics ───────────────────────────────────────────────────
    pub mean_duration_hours: f64,
    pub std_dev_duration_hours: f64,
    pub min_duration_hours: f64,
    pub max_duration_hours: f64,

    // ── Percentiles ───────────────────────────────────────────────────────────
    /// Keyed by confidence level string, e.g. `"0.8"` → cost at P80.
    pub cost_percentiles: std::collections::HashMap<String, f64>,
    /// Keyed by confidence level string, e.g. `"0.8"` → duration hours at P80.
    pub schedule_percentiles: std::collections::HashMap<String, f64>,

    // ── Distributions ─────────────────────────────────────────────────────────
    pub cost_histogram: Vec<HistogramBin>,
    pub schedule_histogram: Vec<HistogramBin>,

    // ── Risk analysis ─────────────────────────────────────────────────────────
    pub risk_contributions: Vec<RiskContributionResult>,
    /// Top risks sorted by |cost_correlation| descending.
    pub tornado: Vec<TornadoEntry>,

    /// ISO datetime (UTC).
    pub computed_at: String,
}

fn default_distribution() -> String { "pert".into() }
fn default_iterations() -> u32 { 10_000 }
fn default_confidence_levels() -> Vec<f64> {
    vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9]
}
