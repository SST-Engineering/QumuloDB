//! QumuloDB Storage Layer
//!
//! Three-tier storage architecture:
//!
//! 1. **Hot tier** — Apache Arrow columnar in-memory record batches.
//!    All active project data lives here for zero-copy kernel access.
//!
//! 2. **Warm tier** — Apache Parquet files on local disk.
//!    Append-only. Every kernel result is persisted as an immutable Parquet file
//!    stamped with transaction time. Enables full audit trail and time-travel queries.
//!
//! 3. **Query tier** — Apache DataFusion SQL engine.
//!    Speaks standard SQL + QQL extensions. Registers Arrow tables and Parquet files
//!    as virtual tables. The `AS OF VALID TIME` and `AS OF TRANSACTION TIME` clauses
//!    are pre-processed into DataFusion logical plans.
//!
//! # Bitemporal model
//!
//! Every stored fact carries four temporal columns:
//! - `valid_from`  / `valid_to`   — when the fact was true in the real world
//! - `txn_from`    / `txn_to`     — when it was recorded in QumuloDB
//!
//! `txn_to = NULL` means the record is current. Closed records have a timestamp.

pub mod error;
pub mod schema;
pub mod session;
pub mod store;

pub use error::StoreError;
pub use store::QDBStore;
