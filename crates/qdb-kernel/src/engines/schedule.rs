//! CPM (Critical Path Method) scheduling engine.
//!
//! Performs a forward pass and backward pass over the activity network to
//! compute early/late dates and float for every activity.

use std::collections::{HashMap, HashSet, VecDeque};

use chrono::{DateTime, Utc};
use qdb_contract::{
    ActivityInput, ActivityResult, DependencyInput, ScheduleRequest, ScheduleResult,
    ScheduleWarning,
};

use crate::{
    calendar::{default_calendar, parse_datetime, RuntimeCalendar},
    KernelError, Result,
};

pub fn run(req: ScheduleRequest) -> Result<ScheduleResult> {
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
        calendars.get(key).unwrap_or_else(|| {
            calendars.get("__default__").unwrap()
        })
    };

    // Parse dates
    let project_start = parse_datetime(&req.project_start)?;
    let data_date = if let Some(dd) = req.data_date {
        parse_datetime(&dd)?
    } else {
        project_start
    };

    // Index activities
    let act_map: HashMap<&str, &ActivityInput> =
        req.activities.iter().map(|a| (a.id.as_str(), a)).collect();

    // Validate dependency references and detect cycles (Kahn's algorithm)
    let mut in_degree: HashMap<&str, usize> = act_map.keys().map(|k| (*k, 0)).collect();
    let mut adj: HashMap<&str, Vec<&str>> = act_map.keys().map(|k| (*k, vec![])).collect();
    let mut warnings: Vec<ScheduleWarning> = Vec::new();

    for dep in &req.dependencies {
        let pred = dep.predecessor_id.as_str();
        let succ = dep.successor_id.as_str();
        if !act_map.contains_key(pred) {
            warnings.push(ScheduleWarning {
                code: "MISSING_PREDECESSOR".into(),
                message: format!("predecessor '{pred}' not found"),
                activity_id: Some(succ.into()),
            });
            continue;
        }
        if !act_map.contains_key(succ) {
            warnings.push(ScheduleWarning {
                code: "MISSING_SUCCESSOR".into(),
                message: format!("successor '{succ}' not found"),
                activity_id: Some(pred.into()),
            });
            continue;
        }
        adj.entry(pred).or_default().push(succ);
        *in_degree.entry(succ).or_default() += 1;
    }

    // Topological sort
    let mut queue: VecDeque<&str> = in_degree
        .iter()
        .filter(|(_, &d)| d == 0)
        .map(|(k, _)| *k)
        .collect();
    let mut topo_order: Vec<&str> = Vec::new();
    let mut visited = 0usize;

    while let Some(node) = queue.pop_front() {
        topo_order.push(node);
        visited += 1;
        for &succ in adj.get(node).unwrap_or(&vec![]) {
            let d = in_degree.entry(succ).or_default();
            *d -= 1;
            if *d == 0 {
                queue.push_back(succ);
            }
        }
    }

    if visited < act_map.len() {
        // Circular dependency — identify a node in the cycle
        let cycle_node = in_degree
            .iter()
            .find(|(_, &d)| d > 0)
            .map(|(k, _)| k.to_string())
            .unwrap_or_default();
        return Err(KernelError::CircularDependency(cycle_node));
    }

    // ── Forward pass ──────────────────────────────────────────────────────────
    let mut early_start: HashMap<&str, DateTime<Utc>> = HashMap::new();
    let mut early_finish: HashMap<&str, DateTime<Utc>> = HashMap::new();

    // Build predecessor map for quick lookup
    let mut pred_deps: HashMap<&str, Vec<&DependencyInput>> = HashMap::new();
    for dep in &req.dependencies {
        pred_deps
            .entry(dep.successor_id.as_str())
            .or_default()
            .push(dep);
    }

    for &id in &topo_order {
        let act = act_map[id];
        let cal = get_cal(act.calendar_id.as_deref());

        // Constraint-based ES
        let mut es = match act.constraint_type.as_str() {
            "MSO" | "SNET" => {
                if let Some(cd) = &act.constraint_date {
                    parse_datetime(cd)?
                } else {
                    project_start
                }
            }
            _ => project_start,
        };

        // Drive from predecessors
        for dep in pred_deps.get(id).into_iter().flatten() {
            let pred = dep.predecessor_id.as_str();
            let pred_es = early_start.get(pred).copied().unwrap_or(project_start);
            let pred_ef = early_finish.get(pred).copied().unwrap_or(project_start);
            let lag = dep.lag_hours;

            let driven = match dep.dep_type.as_str() {
                "SS" => cal.add_working_hours(pred_es, lag.max(0.0)),
                "FF" => {
                    let dur = act_duration(act, cal);
                    let ff_finish = cal.add_working_hours(pred_ef, lag.max(0.0));
                    // ES = FF_finish - duration
                    sub_working_hours(cal, ff_finish, dur)
                }
                "SF" => {
                    let dur = act_duration(act, cal);
                    let sf_finish = cal.add_working_hours(pred_es, lag.max(0.0));
                    sub_working_hours(cal, sf_finish, dur)
                }
                _ => cal.add_working_hours(pred_ef, lag.max(0.0)), // FS default
            };

            if driven > es {
                es = driven;
            }
        }

        // In-progress / completed actual dates override
        if act.status == "in_progress" || act.status == "completed" {
            if let Some(astart) = &act.actual_start {
                if let Ok(dt) = parse_datetime(astart) {
                    es = dt;
                }
            }
        }

        let remaining_hours = remaining_hours(act, cal);
        let ef = cal.add_working_hours(es, remaining_hours);

        early_start.insert(id, es);
        early_finish.insert(id, ef);
    }

    // ── Project finish ────────────────────────────────────────────────────────
    let computed_finish = early_finish.values().copied().fold(project_start, |m, v| m.max(v));
    let project_finish = if let Some(pfc) = &req.project_finish_constraint {
        parse_datetime(pfc)?.max(computed_finish)
    } else {
        computed_finish
    };

    // ── Backward pass ─────────────────────────────────────────────────────────
    let mut late_start: HashMap<&str, DateTime<Utc>> = HashMap::new();
    let mut late_finish: HashMap<&str, DateTime<Utc>> = HashMap::new();

    // Build successor map
    let mut succ_deps: HashMap<&str, Vec<&DependencyInput>> = HashMap::new();
    for dep in &req.dependencies {
        succ_deps
            .entry(dep.predecessor_id.as_str())
            .or_default()
            .push(dep);
    }

    for &id in topo_order.iter().rev() {
        let act = act_map[id];
        let cal = get_cal(act.calendar_id.as_deref());

        let mut lf = project_finish;

        // Constraint-based LF
        match act.constraint_type.as_str() {
            "MFO" | "FNLT" => {
                if let Some(cd) = &act.constraint_date {
                    if let Ok(dt) = parse_datetime(cd) {
                        lf = dt;
                    }
                }
            }
            "FNET" => {
                if let Some(cd) = &act.constraint_date {
                    if let Ok(dt) = parse_datetime(cd) {
                        lf = dt.max(lf);
                    }
                }
            }
            _ => {}
        }

        // Drive from successors
        for dep in succ_deps.get(id).into_iter().flatten() {
            let succ = dep.successor_id.as_str();
            let succ_ls = late_start.get(succ).copied().unwrap_or(project_finish);
            let succ_lf = late_finish.get(succ).copied().unwrap_or(project_finish);
            let lag = dep.lag_hours;

            let driven = match dep.dep_type.as_str() {
                "SS" => {
                    let dur = act_duration(act, cal);
                    let ls_driven = sub_working_hours(cal, succ_ls, lag.max(0.0));
                    cal.add_working_hours(ls_driven, dur)
                }
                "FF" => sub_working_hours(cal, succ_lf, lag.max(0.0)),
                "SF" => {
                    let dur = act_duration(act, cal);
                    let sf_finish = sub_working_hours(cal, succ_ls, lag.max(0.0));
                    cal.add_working_hours(sf_finish, dur)
                }
                _ => sub_working_hours(cal, succ_ls, lag.max(0.0)), // FS
            };

            if driven < lf {
                lf = driven;
            }
        }

        let dur = remaining_hours(act, cal);
        let ls = sub_working_hours(cal, lf, dur);

        late_start.insert(id, ls);
        late_finish.insert(id, lf);
    }

    // ── Float and critical path ───────────────────────────────────────────────
    let mut critical_ids: Vec<(&str, DateTime<Utc>)> = Vec::new();
    let mut negative_float_count = 0usize;

    let mut results: Vec<ActivityResult> = Vec::with_capacity(req.activities.len());

    for act in &req.activities {
        let id = act.id.as_str();
        let cal = get_cal(act.calendar_id.as_deref());

        let es = early_start.get(id).copied();
        let ef = early_finish.get(id).copied();
        let ls = late_start.get(id).copied();
        let lf = late_finish.get(id).copied();

        let total_float = match (es, lf) {
            (Some(es_v), Some(lf_v)) => {
                let ef_v = ef.unwrap_or(es_v);
                let float_h = (lf_v - ef_v).num_minutes() as f64 / 60.0;
                Some(float_h)
            }
            _ => None,
        };

        if let Some(tf) = total_float {
            if tf < 0.0 {
                negative_float_count += 1;
                warnings.push(ScheduleWarning {
                    code: "NEGATIVE_FLOAT".into(),
                    message: format!("activity '{}' has {:.1}h negative float", id, tf),
                    activity_id: Some(id.into()),
                });
            }
        }

        let is_critical = total_float.map(|f| f <= 0.0).unwrap_or(false);
        if is_critical {
            if let Some(es_v) = es {
                critical_ids.push((id, es_v));
            }
        }

        // Deadline check
        if let Some(dl) = &act.deadline {
            if let (Ok(dl_dt), Some(ef_v)) = (parse_datetime(dl), ef) {
                if ef_v > dl_dt {
                    warnings.push(ScheduleWarning {
                        code: "DEADLINE_MISSED".into(),
                        message: format!(
                            "activity '{}' early finish {} exceeds deadline {}",
                            id, ef_v.format("%Y-%m-%dT%H:%M:%S"), dl
                        ),
                        activity_id: Some(id.into()),
                    });
                }
            }
        }

        results.push(ActivityResult {
            id: act.id.clone(),
            name: act.name.clone(),
            duration_hours: act.duration_hours,
            status: act.status.clone(),
            percent_complete: act.percent_complete,
            calendar_id: act.calendar_id.clone(),
            constraint_type: act.constraint_type.clone(),
            constraint_date: act.constraint_date.clone(),
            deadline: act.deadline.clone(),
            actual_start: act.actual_start.clone(),
            actual_finish: act.actual_finish.clone(),
            wbs_code: act.wbs_code.clone(),
            is_milestone: act.is_milestone,
            early_start: es.map(|d| d.format("%Y-%m-%dT%H:%M:%S").to_string()),
            early_finish: ef.map(|d| d.format("%Y-%m-%dT%H:%M:%S").to_string()),
            late_start: ls.map(|d| d.format("%Y-%m-%dT%H:%M:%S").to_string()),
            late_finish: lf.map(|d| d.format("%Y-%m-%dT%H:%M:%S").to_string()),
            total_float_hours: total_float,
            free_float_hours: None,       // TODO: second pass
            independent_float_hours: None, // TODO: second pass
            is_critical,
            scheduled_start: None,
            scheduled_finish: None,
        });
    }

    // Sort critical path by early_start
    critical_ids.sort_by_key(|(_, dt)| *dt);
    let critical_path: Vec<String> = critical_ids.into_iter().map(|(id, _)| id.to_string()).collect();

    let total_duration_hours = {
        let default_cal = get_cal(None);
        default_cal.working_hours_between(project_start, project_finish)
    };

    Ok(ScheduleResult {
        project_id: req.project_id,
        success: true,
        project_start: project_start.format("%Y-%m-%dT%H:%M:%S").to_string(),
        project_finish: project_finish.format("%Y-%m-%dT%H:%M:%S").to_string(),
        data_date: data_date.format("%Y-%m-%dT%H:%M:%S").to_string(),
        total_duration_hours,
        activities: results,
        critical_path,
        warnings,
        negative_float_count,
        computed_at,
    })
}

fn act_duration(act: &ActivityInput, cal: &RuntimeCalendar) -> f64 {
    act.duration_hours
}

fn remaining_hours(act: &ActivityInput, _cal: &RuntimeCalendar) -> f64 {
    if act.status == "completed" {
        return 0.0;
    }
    let pct = act.percent_complete.clamp(0.0, 100.0) / 100.0;
    act.duration_hours * (1.0 - pct)
}

/// Subtract working hours going backwards from `end`.
fn sub_working_hours(cal: &RuntimeCalendar, end: DateTime<Utc>, hours: f64) -> DateTime<Utc> {
    if hours <= 0.0 {
        return end;
    }
    // Binary search approximation: go back `hours * ~1.4` calendar days then forward
    use chrono::Duration;
    let days_estimate = (hours / cal.hours_per_normal_day() * 1.5).ceil() as i64 + 5;
    let search_start = end - Duration::days(days_estimate);
    let forward = cal.add_working_hours(search_start, 0.0); // snap to work start
    // Now scan forward until we consume `(working_hours_between(forward, end) - hours)` hours
    let total_wh = cal.working_hours_between(forward, end);
    if total_wh <= hours {
        return forward;
    }
    cal.add_working_hours(forward, total_wh - hours)
}
