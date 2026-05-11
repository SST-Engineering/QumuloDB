//! QumuloDB Computation Kernel
//!
//! Pure computation — no I/O, no database handles, no async runtime.
//! Every engine takes a typed request and returns a typed result.
//!
//! # Engines
//!
//! - [`engines::schedule`] — CPM/PERT critical-path scheduling
//! - [`engines::evm`]      — Earned Value Management calculation
//! - [`engines::simulation`] — Monte Carlo risk simulation
//! - [`engines::resources`]  — Resource loading and levelling

pub mod calendar;
pub mod engines;
pub mod error;

pub use error::KernelError;

pub type Result<T> = std::result::Result<T, KernelError>;
