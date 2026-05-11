//! QumuloDB — Kernel Contract
//!
//! The single source of truth for every type that crosses the computation kernel boundary.
//!
//! # Design Rules
//!
//! 1. Pure data types only — no I/O, no async, no database handles
//! 2. All dates/datetimes as ISO 8601 strings at the public boundary
//!    (internally converted to `chrono::NaiveDateTime` by the kernel adapters)
//! 3. Every `Option<T>` field has semantic meaning documented on the field
//! 4. All types derive `Serialize`/`Deserialize` — this IS the wire format
//! 5. This crate compiles to WASM unchanged — no platform-specific code
//!
//! # Crate organisation
//!
//! - [`calendar`]   — working-time definitions
//! - [`schedule`]   — CPM scheduling request / result
//! - [`evm`]        — Earned Value Management request / result
//! - [`simulation`] — Monte Carlo simulation request / result
//! - [`resources`]  — Resource scheduling / levelling request / result

pub mod calendar;
pub mod evm;
pub mod resources;
pub mod schedule;
pub mod simulation;

// Re-export the most commonly used types at crate root
pub use calendar::{CalendarException, KernelCalendar};
pub use evm::{EVMPeriodInput, EVMPeriodResult, EVMRequest, EVMResult};
pub use resources::{
    AssignmentInput, LevellingAdjustment, PeriodLoadResult, ResourceInput,
    ResourceLoadResult, ResourceRequest, ResourceResult,
};
pub use schedule::{
    ActivityInput, ActivityResult, DependencyInput, ScheduleRequest, ScheduleResult,
    ScheduleWarning,
};
pub use simulation::{
    HistogramBin, RiskContributionResult, RiskInput, SimulationRequest, SimulationResult,
    TornadoEntry,
};
