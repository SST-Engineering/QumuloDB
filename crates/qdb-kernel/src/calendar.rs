//! Working-time calendar arithmetic for the kernel.

use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, TimeZone, Timelike, Utc, Weekday};
use qdb_contract::KernelCalendar;

use crate::{KernelError, Result};

/// Runtime calendar — wraps contract definition with efficient lookup.
pub struct RuntimeCalendar {
    inner: KernelCalendar,
    /// Sorted set of exception dates.
    exceptions: std::collections::HashMap<NaiveDate, qdb_contract::CalendarException>,
}

impl RuntimeCalendar {
    pub fn new(cal: KernelCalendar) -> Self {
        let mut exceptions = std::collections::HashMap::new();
        for ex in &cal.exceptions {
            if let Ok(d) = NaiveDate::parse_from_str(&ex.date, "%Y-%m-%d") {
                exceptions.insert(d, ex.clone());
            }
        }
        Self { inner: cal, exceptions }
    }

    pub fn id(&self) -> &str {
        &self.inner.calendar_id
    }

    pub fn hours_per_normal_day(&self) -> f64 {
        self.inner.hours_per_day()
    }

    /// Returns working hours available on `date`.
    pub fn hours_on_day(&self, date: NaiveDate) -> f64 {
        if let Some(ex) = self.exceptions.get(&date) {
            return match ex.day_type.as_str() {
                "non_working" => 0.0,
                "half_day" => {
                    let s = ex.work_start_hour as i32 * 60 + ex.work_start_minute as i32;
                    let f = ex.work_finish_hour as i32 * 60 + ex.work_finish_minute as i32;
                    ((f - s).max(0) as f64) / 60.0
                }
                _ => {
                    let s = ex.work_start_hour as i32 * 60 + ex.work_start_minute as i32;
                    let f = ex.work_finish_hour as i32 * 60 + ex.work_finish_minute as i32;
                    ((f - s).max(0) as f64) / 60.0
                }
            };
        }
        let weekday = date.weekday();
        let iso = weekday_to_iso(weekday);
        if self.inner.working_days.contains(&iso) {
            self.hours_per_normal_day()
        } else {
            0.0
        }
    }

    pub fn is_working_day(&self, date: NaiveDate) -> bool {
        self.hours_on_day(date) > 0.0
    }

    /// Advance `start` by `hours` working hours, returning the finish datetime.
    pub fn add_working_hours(&self, start: DateTime<Utc>, hours: f64) -> DateTime<Utc> {
        if hours <= 0.0 {
            return start;
        }

        let mut remaining = hours;
        let mut current = start;

        loop {
            let date = current.date_naive();
            let day_hours = self.hours_on_day(date);

            if day_hours <= 0.0 {
                // Skip to next day work start
                current = next_work_start(self, date + Duration::days(1));
                continue;
            }

            // How many hours remain in today from `current`?
            let work_finish = day_work_finish(self, date);
            let hours_left_today =
                (work_finish - current).num_minutes().max(0) as f64 / 60.0;

            if remaining <= hours_left_today {
                return current + Duration::seconds((remaining * 3600.0) as i64);
            }

            remaining -= hours_left_today;
            current = next_work_start(self, date + Duration::days(1));
        }
    }

    /// Count working hours between two datetimes.
    pub fn working_hours_between(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> f64 {
        if end <= start {
            return 0.0;
        }
        let mut total = 0.0;
        let mut current = start;

        while current < end {
            let date = current.date_naive();
            let day_hours = self.hours_on_day(date);
            if day_hours > 0.0 {
                let day_finish = day_work_finish(self, date).min(end);
                let hours = (day_finish - current).num_minutes().max(0) as f64 / 60.0;
                total += hours;
            }
            let next = next_work_start(self, date + Duration::days(1));
            if next <= current {
                break; // safety
            }
            current = next.min(end);
            if current >= end {
                break;
            }
        }
        total
    }
}

fn weekday_to_iso(w: Weekday) -> u8 {
    match w {
        Weekday::Mon => 0,
        Weekday::Tue => 1,
        Weekday::Wed => 2,
        Weekday::Thu => 3,
        Weekday::Fri => 4,
        Weekday::Sat => 5,
        Weekday::Sun => 6,
    }
}

fn next_work_start(cal: &RuntimeCalendar, mut date: NaiveDate) -> DateTime<Utc> {
    loop {
        if cal.is_working_day(date) {
            let t = NaiveTime::from_hms_opt(
                cal.inner.work_start_hour as u32,
                cal.inner.work_start_minute as u32,
                0,
            )
            .unwrap_or(NaiveTime::from_hms_opt(8, 0, 0).unwrap());
            return Utc.from_utc_datetime(&date.and_time(t));
        }
        date += Duration::days(1);
    }
}

fn day_work_finish(cal: &RuntimeCalendar, date: NaiveDate) -> DateTime<Utc> {
    if let Some(ex) = cal.exceptions.get(&date) {
        let t = NaiveTime::from_hms_opt(
            ex.work_finish_hour as u32,
            ex.work_finish_minute as u32,
            0,
        )
        .unwrap_or(NaiveTime::from_hms_opt(17, 0, 0).unwrap());
        return Utc.from_utc_datetime(&date.and_time(t));
    }
    let t = NaiveTime::from_hms_opt(
        cal.inner.work_finish_hour as u32,
        cal.inner.work_finish_minute as u32,
        0,
    )
    .unwrap_or(NaiveTime::from_hms_opt(17, 0, 0).unwrap());
    Utc.from_utc_datetime(&date.and_time(t))
}

/// Build a default Mon–Fri 08:00–17:00 UTC calendar.
pub fn default_calendar() -> RuntimeCalendar {
    RuntimeCalendar::new(KernelCalendar {
        calendar_id: "__default__".into(),
        name: "Standard".into(),
        timezone: "UTC".into(),
        working_days: vec![0, 1, 2, 3, 4],
        work_start_hour: 8,
        work_start_minute: 0,
        work_finish_hour: 17,
        work_finish_minute: 0,
        exceptions: vec![],
    })
}

pub fn parse_datetime(s: &str) -> Result<DateTime<Utc>> {
    // Try RFC3339 / ISO 8601 with offset first
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&Utc));
    }
    // Try naive datetime (assume UTC)
    if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return Ok(Utc.from_utc_datetime(&ndt));
    }
    if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return Ok(Utc.from_utc_datetime(&ndt));
    }
    // Date-only
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let ndt = d.and_hms_opt(8, 0, 0).unwrap();
        return Ok(Utc.from_utc_datetime(&ndt));
    }
    Err(KernelError::DateParse(s.to_string()))
}
