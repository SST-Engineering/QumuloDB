//! Working-time calendar definitions.

use serde::{Deserialize, Serialize};

/// A single date override in a working calendar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarException {
    /// ISO date string. e.g. `"2026-12-25"`
    pub date: String,

    /// `"non_working"` | `"working"` | `"half_day"`
    pub day_type: String,

    pub work_start_hour: u8,
    pub work_start_minute: u8,
    pub work_finish_hour: u8,
    pub work_finish_minute: u8,
    pub description: String,
}

impl Default for CalendarException {
    fn default() -> Self {
        Self {
            date: String::new(),
            day_type: "non_working".into(),
            work_start_hour: 8,
            work_start_minute: 0,
            work_finish_hour: 17,
            work_finish_minute: 0,
            description: String::new(),
        }
    }
}

/// Working-time definition for a project, resource, or organisation.
///
/// `working_days` lists ISO weekday integers: 0 = Monday, 6 = Sunday.
/// Default `[0, 1, 2, 3, 4]` = Monday–Friday.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelCalendar {
    pub calendar_id: String,
    pub name: String,

    /// IANA timezone string. e.g. `"Europe/London"`, `"America/New_York"`
    #[serde(default = "default_timezone")]
    pub timezone: String,

    /// ISO weekday integers (0 = Monday, 6 = Sunday).
    #[serde(default = "default_working_days")]
    pub working_days: Vec<u8>,

    #[serde(default = "default_start_hour")]
    pub work_start_hour: u8,
    #[serde(default)]
    pub work_start_minute: u8,
    #[serde(default = "default_finish_hour")]
    pub work_finish_hour: u8,
    #[serde(default)]
    pub work_finish_minute: u8,

    #[serde(default)]
    pub exceptions: Vec<CalendarException>,
}

impl KernelCalendar {
    /// Standard working hours on a normal day.
    pub fn hours_per_day(&self) -> f64 {
        let start = self.work_start_hour as i32 * 60 + self.work_start_minute as i32;
        let finish = self.work_finish_hour as i32 * 60 + self.work_finish_minute as i32;
        ((finish - start).max(0) as f64) / 60.0
    }
}

fn default_timezone() -> String { "UTC".into() }
fn default_working_days() -> Vec<u8> { vec![0, 1, 2, 3, 4] }
fn default_start_hour() -> u8 { 8 }
fn default_finish_hour() -> u8 { 17 }
