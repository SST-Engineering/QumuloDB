# QumuloDB Architecture

## The Gentia Parallel

Artemis (1977) was the first tool to treat project management as a language problem — not a spreadsheet problem. It gave `TIME`, `PERIOD`, `CALENDAR`, `ACTIVITY`, `RESOURCE`, and `COST-ACCOUNT` the status of language primitives in the Artemis A2 4GL.

Gentia (1990s) did the same for analytics: a purpose-built multidimensional engine rather than a relational database with OLAP bolted on.

QumuloDB applies this same philosophy to the cloud-native era. The computation kernel IS the product. Every surface — browser, server, mobile, CLI — runs the same Rust kernel.

---

## Crate Graph

```
qdb-contract   (no deps beyond serde/chrono — WASM-safe)
     ↑
qdb-kernel     (pure computation — no I/O)
     ↑
qdb-store      (Arrow + Parquet + DataFusion)
     ↑
qdb-cli        (clap CLI binary)
```

---

## Bitemporal Data Model

All stored facts carry two independent time axes:

```
                    Valid Time axis →
                    (when true in the world)
Transaction     ┌────────────────────────────┐
Time axis   t1  │  v1 ●────────────────────  │  (current record)
↓           t2  │  v1 ●──────── v2 ●──────── │
            t3  │  v1 ●── v2 ●──── v3 ●───── │
                └────────────────────────────┘
```

Columns on every table:
- `valid_from` / `valid_to` — when the fact was true in the real world
- `txn_from` / `txn_to` — when it was recorded in QumuloDB

A `NULL` `txn_to` means the record is currently active.

### Time-travel queries

```sql
-- What did we think the schedule looked like on 1 March?
SELECT * FROM qdb_activities
WHERE project_id = 'P1'
AS OF VALID TIME '2026-03-01';

-- What did we record on 15 January, regardless of valid time?
SELECT * FROM qdb_activities
WHERE project_id = 'P1'
AS OF TRANSACTION TIME '2026-01-15';

-- Fully bitemporal — both axes pinned
SELECT * FROM qdb_activities
WHERE project_id = 'P1'
AS OF VALID TIME '2026-03-01' TRANSACTION TIME '2026-01-15';
```

---

## Kernel Contract

The `qdb-contract` crate is the single source of truth for every type that crosses the kernel boundary:

- Pure Rust structs — no methods beyond trivial helpers
- All dates as ISO 8601 strings at the boundary
- `#[derive(Serialize, Deserialize)]` — the structs ARE the wire format
- No `std::io`, no `async`, no platform-specific code
- Compiles to `wasm32-unknown-unknown` unchanged

---

## Storage Tiers

### Hot tier — Apache Arrow

Apache Arrow provides zero-copy columnar in-memory access. Every kernel result is converted to Arrow `RecordBatch`es and held in the `ProjectSession`. The kernel can read directly from Arrow memory without deserialization.

### Warm tier — Apache Parquet

Every kernel result is written as an immutable Parquet file:
```
qdb_data/
  schedules/     {project_id}_{txn_timestamp}.json  → future: .parquet
  evm/
  simulations/
  resources/
```

Parquet enables efficient column-pruned reads for historical analysis without loading full rows.

### Query tier — Apache DataFusion

DataFusion is a SQL query engine framework. QumuloDB registers Arrow tables and Parquet scans as virtual tables and extends the SQL dialect with QQL PM-domain clauses.

DataFusion was chosen because:
- It is an engine framework (building material), not a finished product
- Extensible logical plan rewrites for `AS OF VALID TIME`
- Columnar execution on Arrow memory — zero-copy
- Compiles to Linux/macOS/Windows without external dependencies

---

## WASM Target

`qdb-contract` and `qdb-kernel` compile to `wasm32-unknown-unknown`:

```
cargo check -p qdb-contract --target wasm32-unknown-unknown
cargo check -p qdb-kernel   --target wasm32-unknown-unknown
```

The compiled WASM module is loaded into a browser Web Worker by Q-Surface (Next.js). This gives the UI zero-round-trip recalculation — the full CPM engine runs locally in the browser without an API call.

`qdb-store` is native-only; the browser uses in-memory Arrow only.
