//! Arrow schema definitions for all QumuloDB tables.
//!
//! Every table follows the bitemporal convention:
//! `valid_from`, `valid_to`, `txn_from`, `txn_to` (Utf8 ISO datetimes).
//! `txn_to = NULL` marks the current record.

use arrow_schema::{DataType, Field, Schema};
use std::sync::Arc;

pub fn schedule_snapshots_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("snapshot_id", DataType::Utf8, false),
        Field::new("project_id", DataType::Utf8, false),
        Field::new("project_start", DataType::Utf8, false),
        Field::new("project_finish", DataType::Utf8, false),
        Field::new("data_date", DataType::Utf8, false),
        Field::new("total_duration_hours", DataType::Float64, false),
        Field::new("success", DataType::Boolean, false),
        Field::new("negative_float_count", DataType::Int64, false),
        Field::new("critical_path_json", DataType::Utf8, false),
        Field::new("warnings_json", DataType::Utf8, false),
        Field::new("computed_at", DataType::Utf8, false),
        // Bitemporal columns
        Field::new("valid_from", DataType::Utf8, false),
        Field::new("valid_to", DataType::Utf8, true),
        Field::new("txn_from", DataType::Utf8, false),
        Field::new("txn_to", DataType::Utf8, true),
    ]))
}

pub fn activities_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("activity_id", DataType::Utf8, false),
        Field::new("snapshot_id", DataType::Utf8, false),
        Field::new("project_id", DataType::Utf8, false),
        Field::new("name", DataType::Utf8, false),
        Field::new("duration_hours", DataType::Float64, false),
        Field::new("status", DataType::Utf8, false),
        Field::new("percent_complete", DataType::Float64, false),
        Field::new("is_critical", DataType::Boolean, false),
        Field::new("is_milestone", DataType::Boolean, false),
        Field::new("early_start", DataType::Utf8, true),
        Field::new("early_finish", DataType::Utf8, true),
        Field::new("late_start", DataType::Utf8, true),
        Field::new("late_finish", DataType::Utf8, true),
        Field::new("total_float_hours", DataType::Float64, true),
        Field::new("free_float_hours", DataType::Float64, true),
        Field::new("wbs_code", DataType::Utf8, true),
        Field::new("constraint_type", DataType::Utf8, false),
        // Bitemporal columns
        Field::new("valid_from", DataType::Utf8, false),
        Field::new("valid_to", DataType::Utf8, true),
        Field::new("txn_from", DataType::Utf8, false),
        Field::new("txn_to", DataType::Utf8, true),
    ]))
}

pub fn evm_series_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("evm_id", DataType::Utf8, false),
        Field::new("project_id", DataType::Utf8, false),
        Field::new("data_date", DataType::Utf8, false),
        Field::new("bac", DataType::Float64, false),
        Field::new("pv", DataType::Float64, false),
        Field::new("ev", DataType::Float64, false),
        Field::new("ac", DataType::Float64, false),
        Field::new("cv", DataType::Float64, false),
        Field::new("sv", DataType::Float64, false),
        Field::new("cpi", DataType::Float64, false),
        Field::new("spi", DataType::Float64, false),
        Field::new("eac", DataType::Float64, false),
        Field::new("etc", DataType::Float64, false),
        Field::new("vac", DataType::Float64, false),
        Field::new("tcpi", DataType::Float64, false),
        Field::new("percent_complete", DataType::Float64, false),
        Field::new("percent_spent", DataType::Float64, false),
        Field::new("eac_method", DataType::Utf8, false),
        Field::new("computed_at", DataType::Utf8, false),
        Field::new("valid_from", DataType::Utf8, false),
        Field::new("valid_to", DataType::Utf8, true),
        Field::new("txn_from", DataType::Utf8, false),
        Field::new("txn_to", DataType::Utf8, true),
    ]))
}

pub fn simulations_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("simulation_id", DataType::Utf8, false),
        Field::new("project_id", DataType::Utf8, false),
        Field::new("iterations", DataType::Int64, false),
        Field::new("mean_cost", DataType::Float64, false),
        Field::new("std_dev_cost", DataType::Float64, false),
        Field::new("p50_cost", DataType::Float64, false),
        Field::new("p80_cost", DataType::Float64, false),
        Field::new("p90_cost", DataType::Float64, false),
        Field::new("mean_duration_hours", DataType::Float64, false),
        Field::new("p50_duration_hours", DataType::Float64, false),
        Field::new("p80_duration_hours", DataType::Float64, false),
        Field::new("tornado_json", DataType::Utf8, false),
        Field::new("computed_at", DataType::Utf8, false),
        Field::new("valid_from", DataType::Utf8, false),
        Field::new("valid_to", DataType::Utf8, true),
        Field::new("txn_from", DataType::Utf8, false),
        Field::new("txn_to", DataType::Utf8, true),
    ]))
}

pub fn resource_loads_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("load_id", DataType::Utf8, false),
        Field::new("project_id", DataType::Utf8, false),
        Field::new("resource_id", DataType::Utf8, false),
        Field::new("resource_name", DataType::Utf8, false),
        Field::new("period_start", DataType::Utf8, false),
        Field::new("period_end", DataType::Utf8, false),
        Field::new("assigned_hours", DataType::Float64, false),
        Field::new("available_hours", DataType::Float64, false),
        Field::new("utilisation", DataType::Float64, false),
        Field::new("overallocation_hours", DataType::Float64, false),
        Field::new("computed_at", DataType::Utf8, false),
        Field::new("valid_from", DataType::Utf8, false),
        Field::new("valid_to", DataType::Utf8, true),
        Field::new("txn_from", DataType::Utf8, false),
        Field::new("txn_to", DataType::Utf8, true),
    ]))
}
