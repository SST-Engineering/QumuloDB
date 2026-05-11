//! Earned Value Management calculation engine.

use chrono::Utc;
use qdb_contract::{EVMPeriodResult, EVMRequest, EVMResult};

use crate::{KernelError, Result};

pub fn run(req: EVMRequest) -> Result<EVMResult> {
    let computed_at = Utc::now().to_rfc3339();

    if req.periods.is_empty() {
        return Err(KernelError::InvalidInput("periods must not be empty".into()));
    }

    // Find the period at or before data_date
    let active = req
        .periods
        .iter()
        .filter(|p| p.period_end.as_str() <= req.data_date.as_str())
        .last()
        .unwrap_or(&req.periods[0]);

    let pv = active.planned_value;
    let ev = active.earned_value;
    let ac = active.actual_cost;
    let bac = req.bac;

    let cv = ev - ac;
    let sv = ev - pv;
    let cv_pct = if ev != 0.0 { cv / ev * 100.0 } else { 0.0 };
    let sv_pct = if pv != 0.0 { sv / pv * 100.0 } else { 0.0 };
    let cpi = if ac != 0.0 { ev / ac } else { 1.0 };
    let spi = if pv != 0.0 { ev / pv } else { 1.0 };

    let eac = compute_eac(&req.eac_method, req.manual_eac, bac, ev, ac, cpi)?;
    let etc = eac - ac;
    let vac = bac - eac;
    let tcpi = if (bac - ac) != 0.0 { (bac - ev) / (bac - ac) } else { 1.0 };
    let percent_complete = if bac != 0.0 { ev / bac * 100.0 } else { 0.0 };
    let percent_spent = if bac != 0.0 { ac / bac * 100.0 } else { 0.0 };

    // S-curve periods
    let mut periods: Vec<EVMPeriodResult> = Vec::with_capacity(req.periods.len());
    let mut prev_pv = 0.0_f64;
    let mut prev_ev = 0.0_f64;
    let mut prev_ac = 0.0_f64;

    for p in &req.periods {
        let ppv = p.planned_value - prev_pv;
        let pev = p.earned_value - prev_ev;
        let pac = p.actual_cost - prev_ac;
        let period_cpi = if pac != 0.0 { Some(pev / pac) } else { None };
        let period_spi = if ppv != 0.0 { Some(pev / ppv) } else { None };

        periods.push(EVMPeriodResult {
            period_start: p.period_start.clone(),
            period_end: p.period_end.clone(),
            cumulative_pv: p.planned_value,
            cumulative_ev: p.earned_value,
            cumulative_ac: p.actual_cost,
            period_pv: ppv,
            period_ev: pev,
            period_ac: pac,
            period_cpi,
            period_spi,
        });

        prev_pv = p.planned_value;
        prev_ev = p.earned_value;
        prev_ac = p.actual_cost;
    }

    Ok(EVMResult {
        project_id: req.project_id,
        data_date: req.data_date,
        bac,
        pv,
        ev,
        ac,
        cv,
        sv,
        cv_pct,
        sv_pct,
        cpi,
        spi,
        eac,
        etc,
        vac,
        tcpi,
        percent_complete,
        percent_spent,
        eac_method: req.eac_method,
        periods,
        computed_at,
    })
}

fn compute_eac(
    method: &str,
    manual_eac: Option<f64>,
    bac: f64,
    ev: f64,
    ac: f64,
    cpi: f64,
) -> Result<f64> {
    match method {
        "cpi" => {
            if cpi == 0.0 {
                return Err(KernelError::InvalidInput("CPI is zero — cannot compute EAC".into()));
            }
            Ok(bac / cpi)
        }
        "spi" => {
            // EAC = AC + (BAC - EV) — uses SPI to forecast remaining but simplified
            Ok(ac + (bac - ev))
        }
        "remaining" => Ok(ac + (bac - ev)),
        "manual" => manual_eac
            .ok_or_else(|| KernelError::InvalidInput("manual_eac required when eac_method = manual".into())),
        _ => Err(KernelError::InvalidInput(format!("unknown eac_method '{method}'"))),
    }
}
