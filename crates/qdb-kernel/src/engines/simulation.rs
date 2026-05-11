//! Monte Carlo risk simulation engine.

use std::collections::HashMap;

use chrono::Utc;
use rand::{Rng, SeedableRng};
use rand_distr::{Beta, Distribution, Triangular, Uniform};
use qdb_contract::{
    HistogramBin, RiskContributionResult, SimulationRequest, SimulationResult, TornadoEntry,
};

use crate::{KernelError, Result};

pub fn run(req: SimulationRequest) -> Result<SimulationResult> {
    let computed_at = Utc::now().to_rfc3339();
    let n = req.iterations as usize;

    let mut rng: rand::rngs::SmallRng = match req.seed {
        Some(s) => rand::rngs::SmallRng::seed_from_u64(s),
        None => rand::rngs::SmallRng::from_entropy(),
    };

    let mut total_costs = Vec::with_capacity(n);
    let mut total_durations = Vec::with_capacity(n);

    // Per-risk materialisation tracking
    let nrisks = req.risks.len();
    let mut risk_cost_impacts: Vec<Vec<f64>> = vec![Vec::with_capacity(n); nrisks];
    let mut risk_sched_impacts: Vec<Vec<f64>> = vec![Vec::with_capacity(n); nrisks];
    let mut risk_materialised: Vec<u32> = vec![0; nrisks];

    for _ in 0..n {
        let mut iter_cost = req.base_cost;
        let mut iter_dur = req.base_duration_hours;

        for (i, risk) in req.risks.iter().enumerate() {
            let prob = risk.probability.clamp(0.0, 1.0);
            let p: f64 = rng.gen();
            if p > prob {
                risk_cost_impacts[i].push(0.0);
                risk_sched_impacts[i].push(0.0);
                continue;
            }

            risk_materialised[i] += 1;

            let cost_impact = sample_distribution(
                &mut rng,
                &risk.distribution,
                risk.min_cost,
                risk.ml_cost,
                risk.max_cost,
            )?;
            let sched_impact = sample_distribution(
                &mut rng,
                &risk.distribution,
                risk.min_schedule_hours,
                risk.ml_schedule_hours,
                risk.max_schedule_hours,
            )?;

            risk_cost_impacts[i].push(cost_impact);
            risk_sched_impacts[i].push(sched_impact);
            iter_cost += cost_impact;
            iter_dur += sched_impact;
        }

        total_costs.push(iter_cost);
        total_durations.push(iter_dur);
    }

    // ── Statistics ────────────────────────────────────────────────────────────
    total_costs.sort_by(f64::total_cmp);
    total_durations.sort_by(f64::total_cmp);

    let mean_cost = mean(&total_costs);
    let mean_dur = mean(&total_durations);
    let std_dev_cost = std_dev(&total_costs, mean_cost);
    let std_dev_dur = std_dev(&total_durations, mean_dur);

    // ── Percentiles ───────────────────────────────────────────────────────────
    let mut cost_percentiles = HashMap::new();
    let mut schedule_percentiles = HashMap::new();
    for &cl in &req.confidence_levels {
        let key = format!("{:.1}", cl * 100.0);
        cost_percentiles.insert(key.clone(), percentile(&total_costs, cl));
        schedule_percentiles.insert(key, percentile(&total_durations, cl));
    }

    // ── Histograms ────────────────────────────────────────────────────────────
    let cost_histogram = build_histogram(&total_costs, 20);
    let schedule_histogram = build_histogram(&total_durations, 20);

    // ── Risk correlations ─────────────────────────────────────────────────────
    let mut risk_contributions = Vec::with_capacity(nrisks);
    let mut tornado = Vec::new();

    for (i, risk) in req.risks.iter().enumerate() {
        let mean_ci = mean(&risk_cost_impacts[i]);
        let mean_si = mean(&risk_sched_impacts[i]);
        let cost_corr = pearson(&risk_cost_impacts[i], &total_costs);
        let sched_corr = pearson(&risk_sched_impacts[i], &total_durations);

        risk_contributions.push(RiskContributionResult {
            risk_id: risk.risk_id.clone(),
            name: risk.name.clone(),
            mean_cost_impact: mean_ci,
            mean_schedule_impact_hours: mean_si,
            cost_correlation: cost_corr,
            schedule_correlation: sched_corr,
            materialised_count: risk_materialised[i],
        });

        tornado.push(TornadoEntry {
            risk_id: risk.risk_id.clone(),
            name: risk.name.clone(),
            cost_correlation: cost_corr,
            schedule_correlation: sched_corr,
        });
    }

    tornado.sort_by(|a, b| {
        b.cost_correlation
            .abs()
            .partial_cmp(&a.cost_correlation.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(SimulationResult {
        project_id: req.project_id,
        iterations: req.iterations,
        mean_cost,
        std_dev_cost,
        min_cost: total_costs.first().copied().unwrap_or(0.0),
        max_cost: total_costs.last().copied().unwrap_or(0.0),
        mean_duration_hours: mean_dur,
        std_dev_duration_hours: std_dev_dur,
        min_duration_hours: total_durations.first().copied().unwrap_or(0.0),
        max_duration_hours: total_durations.last().copied().unwrap_or(0.0),
        cost_percentiles,
        schedule_percentiles,
        cost_histogram,
        schedule_histogram,
        risk_contributions,
        tornado,
        computed_at,
    })
}

fn sample_distribution(
    rng: &mut impl Rng,
    dist: &str,
    min: f64,
    ml: f64,
    max: f64,
) -> Result<f64> {
    if min >= max {
        return Ok(ml);
    }
    match dist {
        "fixed" => Ok(ml),
        "uniform" => {
            let d = Uniform::new(min, max);
            Ok(d.sample(rng))
        }
        "triangular" => {
            let ml_clamped = ml.clamp(min, max);
            let d = Triangular::new(min, max, ml_clamped)
                .map_err(|e| KernelError::InvalidInput(e.to_string()))?;
            Ok(d.sample(rng))
        }
        "pert" | _ => {
            // PERT via Beta distribution: α = 1 + 4*(ml-min)/(max-min), β = 1 + 4*(max-ml)/(max-min)
            let range = max - min;
            if range < 1e-10 {
                return Ok(ml);
            }
            let alpha = 1.0 + 4.0 * (ml - min) / range;
            let beta_param = 1.0 + 4.0 * (max - ml) / range;
            let d = Beta::new(alpha.max(0.01), beta_param.max(0.01))
                .map_err(|e| KernelError::InvalidInput(e.to_string()))?;
            Ok(min + d.sample(rng) * range)
        }
    }
}

fn mean(v: &[f64]) -> f64 {
    if v.is_empty() { return 0.0; }
    v.iter().sum::<f64>() / v.len() as f64
}

fn std_dev(v: &[f64], mean: f64) -> f64 {
    if v.len() < 2 { return 0.0; }
    let variance = v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (v.len() - 1) as f64;
    variance.sqrt()
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() { return 0.0; }
    let idx = (p * sorted.len() as f64).ceil() as usize;
    sorted[(idx.saturating_sub(1)).min(sorted.len() - 1)]
}

fn build_histogram(sorted: &[f64], bins: usize) -> Vec<HistogramBin> {
    if sorted.is_empty() { return vec![]; }
    let min = *sorted.first().unwrap();
    let max = *sorted.last().unwrap();
    if (max - min).abs() < 1e-10 {
        return vec![HistogramBin {
            bin_start: min,
            bin_end: max,
            count: sorted.len() as u32,
            frequency: 1.0,
            cumulative: 1.0,
        }];
    }
    let width = (max - min) / bins as f64;
    let n = sorted.len() as f64;
    let mut result = Vec::with_capacity(bins);
    let mut cumulative = 0.0;

    for i in 0..bins {
        let start = min + i as f64 * width;
        let end = if i == bins - 1 { max + 1e-10 } else { start + width };
        let count = sorted.iter().filter(|&&x| x >= start && x < end).count() as u32;
        let freq = count as f64 / n;
        cumulative += freq;
        result.push(HistogramBin {
            bin_start: start,
            bin_end: end - if i == bins - 1 { 1e-10 } else { 0.0 },
            count,
            frequency: freq,
            cumulative,
        });
    }
    result
}

fn pearson(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len().min(y.len());
    if n < 2 { return 0.0; }
    let mx = mean(x);
    let my = mean(y);
    let num: f64 = x.iter().zip(y.iter()).map(|(a, b)| (a - mx) * (b - my)).sum();
    let dx: f64 = x.iter().map(|a| (a - mx).powi(2)).sum::<f64>().sqrt();
    let dy: f64 = y.iter().map(|b| (b - my).powi(2)).sum::<f64>().sqrt();
    if dx < 1e-12 || dy < 1e-12 { 0.0 } else { num / (dx * dy) }
}
