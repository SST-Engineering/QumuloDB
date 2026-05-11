pub mod evm;
pub mod resources;
pub mod schedule;

// Monte Carlo uses rand::from_entropy() which requires getrandom/js on WASM.
// Simulation runs natively or server-side; the browser uses CPM/EVM/resources.
#[cfg(not(target_arch = "wasm32"))]
pub mod simulation;
