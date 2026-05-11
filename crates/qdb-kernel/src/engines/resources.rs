//! Resource loading and levelling engine.

use std::collections::HashMap;

use chrono::{DateTime, Datelike, Duration, Utc};
use qdb_contract::{
    LevellingAdjustment, PeriodLoadResult, ResourceInput, ResourceLoadResult, ResourceRequest,
    ResourceResult,
};

use crate::{
    calendar::{default_calendar, parse_datetime, RuntimeCalendar},
    Result,
};

pub fn run(req: ResourceRequest) -> Result<ResourceResult> {
    let computed_at = Utc::now().to_rfc3339();

    // Build calendar lookup
    let mut calendars: HashMap<String, RuntimeCalendar> = req
        .calendars
        .into_iter()
        .map(|c| (c.calendar_id.clone(), RuntimeCalendar::new(c)))
        .collect();
    let default_cal_id = req.default_calendar_id.unwrap_or("__default__".into());
    if !calendars.contains_key(&default_cal_id) {
        calendars.insert("__default__".into(), default_calendar());
    }

    let get_cal = |id: Option<&str>| -> &RuntimeCalendar {
        let key = id.unwrap_or(&default_cal_id);
        calendars.get(key).unwrap_or_else(|| calendars.get("__default__").unwrap())
    };

    // Determine report window
    let (report_start, report_end) = report_window(&req.activities, &req.report_start, &req.report_end)?;

    // Build activity date index: activity_id → (start, finish)
    let mut act_dates: HashMap<String, (DateTime<Utc>, DateTime<Utc>)> = HashMap::new();
    for act in &req.activities {
        let start_str = act.scheduled_start.as_deref()
            .or(act.early_start.as_deref())
            .unwrap_or(&req.activities[0].early_start.as_deref().unwrap_or(""));
        let finish_str = act.scheduled_finish.as_deref()
            .or(act.early_finish.as_deref())
            .unwrap_or("");

        if let (Ok(s), Ok(f)) = (parse_datetime(start_str), parse_datetime(finish_str)) {
            act_dates.insert(act.id.clone(), (s, f));
        }
    }

    // Build assignment index: resource_id → Vec<assignment>
    let mut res_assignments: HashMap<String, Vec<&qdb_contract::AssignmentInput>> = HashMap::new();
    for a in &req.assignments {
        res_assignments.entry(a.resource_id.clone()).or_default().push(a);
    }

    // Build period boundaries
    let periods = build_periods(report_start, report_end, &req.period_granularity);

    // Per-resource load
    let mut resource_results = Vec::new();
    let mut total_cost = 0.0_f64;

    let resource_map: HashMap<&str, &ResourceInput> =
        req.resources.iter().map(|r| (r.resource_id.as_str(), r)).collect();

    for resource in &req.resources {
        let cal = get_cal(resource.calendar_id.as_deref());
        let assignments = res_assignments.get(&resource.resource_id).cloned().unwrap_or_default();

        let mut period_results = Vec::with_capacity(periods.len());
        let mut peak_util = 0.0_f64;
        let mut total_assigned = 0.0_f64;
        let mut total_available = 0.0_f64;
        let mut is_over = false;

        for (ps, pe) in &periods {
            let avail = cal.working_hours_between(*ps, *pe) * resource.max_units;

            let mut assigned = 0.0_f64;
            for asgn in &assignments {
                let units = asgn.units.clamp(0.0, resource.max_units);
                if let Some(&(act_start, act_finish)) = act_dates.get(&asgn.activity_id) {
                    // Overlap between period and activity
                    let overlap_start = act_start.max(*ps);
                    let overlap_end = act_finish.min(*pe);
                    if overlap_end > overlap_start {
                        assigned += cal.working_hours_between(overlap_start, overlap_end) * units;
                    }
                }
            }

            let utilisation = if avail > 0.0 { assigned / avail } else { 0.0 };
            let overalloc = (assigned - avail).max(0.0);
            if utilisation > 1.0 { is_over = true; }
            if utilisation > peak_util { peak_util = utilisation; }
            total_assigned += assigned;
            total_available += avail;
            total_cost += assigned * resource.cost_per_hour;

            period_results.push(PeriodLoadResult {
                period_start: ps.format("%Y-%m-%dT%H:%M:%S").to_string(),
                period_end: pe.format("%Y-%m-%dT%H:%M:%S").to_string(),
                assigned_hours: assigned,
                available_hours: avail,
                utilisation,
                overallocation_hours: overalloc,
            });
        }

        resource_results.push(ResourceLoadResult {
            resource_id: resource.resource_id.clone(),
            name: resource.name.clone(),
            resource_type: resource.resource_type.clone(),
            max_units: resource.max_units,
            total_assigned_hours: total_assigned,
            total_available_hours: total_available,
            peak_utilisation: peak_util,
            is_over_allocated: is_over,
            periods: period_results,
        });
    }

    let has_overallocation = resource_results.iter().any(|r| r.is_over_allocated);
    let levelling_adjustments: Vec<LevellingAdjustment> = Vec::new();
    // TODO: resource levelling algorithm (delay activities to resolve overallocation)

    Ok(ResourceResult {
        project_id: req.project_id,
        resources: resource_results,
        levelling_adjustments,
        has_overallocation,
        total_cost,
        computed_at,
    })
}

fn report_window(
    activities: &[qdb_contract::ActivityResult],
    report_start: &Option<String>,
    report_end: &Option<String>,
) -> Result<(DateTime<Utc>, DateTime<Utc>)> {
    let rs = if let Some(s) = report_start {
        parse_datetime(s)?
    } else {
        activities
            .iter()
            .filter_map(|a| a.early_start.as_deref().and_then(|s| parse_datetime(s).ok()))
            .min()
            .unwrap_or_else(Utc::now)
    };

    let re = if let Some(e) = report_end {
        parse_datetime(e)?
    } else {
        activities
            .iter()
            .filter_map(|a| a.early_finish.as_deref().and_then(|s| parse_datetime(s).ok()))
            .max()
            .unwrap_or_else(|| rs + Duration::days(30))
    };

    Ok((rs, re))
}

fn build_periods(
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    granularity: &str,
) -> Vec<(DateTime<Utc>, DateTime<Utc>)> {
    let mut periods = Vec::new();
    let mut current = start;

    while current < end {
        let next = match granularity {
            "weekly" => current + Duration::weeks(1),
            "monthly" => {
                let y = current.year();
                let m = current.month();
                let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
                current
                    .with_year(ny)
                    .and_then(|d| d.with_month(nm))
                    .unwrap_or(current + Duration::days(30))
            }
            _ => current + Duration::days(1), // daily
        };
        let period_end = next.min(end);
        periods.push((current, period_end));
        current = next;
        if current >= end { break; }
    }

    periods
}
