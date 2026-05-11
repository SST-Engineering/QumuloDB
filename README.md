# QumuloDB

> The purpose-built project management database engine for the 21st century.

QumuloDB is an original Rust database engine designed from first principles for project management data — the same ambition that Artemis brought to the mainframe era and Gentia brought to client-server, now rebuilt for the cloud-native, local-first, AI-augmented world.

It is not built on someone else's product. It uses Apache Arrow, Parquet, and DataFusion as low-level building materials — the way a car manufacturer uses steel and glass, not finished vehicles.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                           Q-Surface (Next.js)                        │
│              hosts qdb-kernel compiled to WASM in Web Workers         │
└────────────────────────────────┬────────────────────────────────────┘
                                 │ QQL / JSON
┌────────────────────────────────▼────────────────────────────────────┐
│                        Q-API (Kotlin + Ktor)                          │
│              REST/WebSocket gateway · auth · project state            │
└────────────────────────────────┬────────────────────────────────────┘
                                 │ FFI / JSON
┌────────────────────────────────▼────────────────────────────────────┐
│                      qdb-kernel  (this repo)                          │
│  CPM scheduling · EVM · Monte Carlo · Resource levelling · Foresight  │
└──────────┬───────────────────────────────────┬──────────────────────┘
           │ Arrow / Parquet                   │ WASM
    ┌──────▼──────┐                    ┌───────▼──────┐
    │  qdb-store  │                    │   Browser    │
    │ Arrow hot   │                    │  Web Worker  │
    │ Parquet warm│                    └──────────────┘
    │ DataFusion  │
    └──────┬──────┘
           │
    ┌──────▼──────┐
    │  Q-State    │
    │ PostgreSQL  │
    │   Redis     │
    └─────────────┘
```

### Crates

| Crate | Purpose |
|-------|---------|
| `qdb-contract` | The single source of truth for every type that crosses the kernel boundary. Pure data — no I/O. Compiles to WASM unchanged. |
| `qdb-kernel` | Computation engines: CPM/PERT, EVM, Monte Carlo, Resource levelling. Pure functions — no I/O. |
| `qdb-store` | Storage layer: Apache Arrow in-memory hot tier, Apache Parquet on-disk warm tier, Apache DataFusion SQL query engine. Bitemporal data model. |
| `qdb-cli` | `qdb` command-line tool for running kernel computations from JSON files. |

---

## Five ahead-of-time design properties

1. **Local-first + CRDT sync** — projects work fully offline; sync is a background process
2. **Bitemporal data model** — every fact carries `valid_time` (when true in the world) and `transaction_time` (when recorded). Full audit trail. Time-travel queries.
3. **Continuous Foresight** — event-driven recalculation; every change triggers an immediate forecast update
4. **Invisible AI inference** — risk scoring, delay prediction, and critical-path foresight live inside the kernel, not bolted on
5. **Graph-native project model** — activities, dependencies, calendars, resources, and costs are first-class graph nodes; no impedance mismatch with relational storage

---

## Kernel Engines

### CPM Scheduling (`qdb-kernel::engines::schedule`)

Full Critical Path Method with:
- Forward and backward pass
- FS / SS / FF / SF dependency types with lag/lead
- Activity constraints: ASAP, ALAP, MSO, MFO, SNET, SNLT, FNET, FNLT
- Working-time calendars with timezone awareness and exception dates
- Total float, free float, critical path extraction
- In-progress activity handling (actual start / percent complete)
- Deadline warnings, negative float detection

### EVM Calculation (`qdb-kernel::engines::evm`)

Earned Value Management with:
- PV, EV, AC at data date
- CV, SV, CPI, SPI, CV%, SV%
- EAC forecasting: CPI method, SPI method, remaining-work method, manual override
- ETC, VAC, TCPI, percent complete, percent spent
- S-curve period breakdown with period-level CPI/SPI

### Monte Carlo Simulation (`qdb-kernel::engines::simulation`)

Risk simulation with:
- Triangular, PERT-Beta, Uniform, and Fixed distributions
- Configurable iterations and random seed
- Percentile outputs at any confidence level
- Cost and schedule histograms
- Per-risk Pearson correlation (tornado chart)

### Resource Loading & Levelling (`qdb-kernel::engines::resources`)

Resource management with:
- Per-resource load profiles (daily / weekly / monthly periods)
- Over-allocation detection and utilisation ratios
- Total cost calculation
- Levelling algorithm (Phase 2)

---

## Storage — QumuloDB Store

The storage layer implements a bitemporal append-only model using:

- **Apache Arrow** record batches for zero-copy in-memory access
- **Apache Parquet** for immutable on-disk persistence with column pruning
- **Apache DataFusion** for SQL queries — registered Arrow tables + Parquet scan

Every stored record carries:
```
valid_from  / valid_to   — when the fact was true in the real world
txn_from    / txn_to     — when it was recorded in QumuloDB
```
`txn_to = NULL` marks the current record. Closing a record stamps it with the current transaction time before inserting the new version.

---

## QQL — Qumulo Query Language

QQL is standard SQL extended with project management semantics, inspired by the Artemis A2 language where `TIME`, `PERIOD`, `CALENDAR`, `ACTIVITY`, and `RESOURCE` were first-class language primitives.

```sql
-- Critical activities with negative float at a specific point in time
SELECT activity_id, name, total_float_hours
FROM   qdb_activities
WHERE  project_id = 'PROJ-001'
  AND  is_critical = true
AS OF VALID TIME '2026-03-01'
ORDER BY total_float_hours ASC;

-- EVM trend — cost performance over time
SELECT data_date, cpi, spi, eac
FROM   qdb_evm_series
WHERE  project_id = 'PROJ-001'
  AND  data_date BETWEEN '2026-01-01' AND '2026-06-30'
ORDER BY data_date;

-- Activities in a schedule period
SELECT activity_id, name, early_start, early_finish
FROM   qdb_activities
WHERE  SCHEDULE_PERIOD OVERLAPS ('2026-Q2');
```

Full QQL reference: [`docs/qql.md`](docs/qql.md)

---

## Getting Started

### Prerequisites

- Rust 1.78+ (`rustup update stable`)
- Cargo

### Build

```bash
cargo build
```

### Run the CLI

```bash
# Schedule a project from JSON
echo '{"project_id":"p1","project_start":"2026-01-06","activities":[{"id":"A","name":"Design","duration_hours":40},{"id":"B","name":"Build","duration_hours":80}],"dependencies":[{"predecessor_id":"A","successor_id":"B"}]}' \
  | cargo run --bin qdb -- schedule

# EVM calculation
cargo run --bin qdb -- evm --input evm_request.json --output evm_result.json

# Monte Carlo simulation
cargo run --bin qdb -- simulate --input risks.json

# With persistence
cargo run --bin qdb -- schedule --input schedule.json --persist
```

### Run tests

```bash
cargo test
```

---

## Roadmap

| Phase | Focus |
|-------|-------|
| 0 ✅ | Contract types + kernel scaffold (this PR) |
| 1 | Full CPM engine with calendar arithmetic |
| 2 | Resource levelling algorithm |
| 3 | Arrow/Parquet persistence — full bitemporal writes |
| 4 | DataFusion QQL extensions (`AS OF VALID TIME`, `SCHEDULE_PERIOD OVERLAPS`) |
| 5 | WASM build target + browser Web Worker harness |
| 6 | Foresight engine — ML-driven delay prediction |
| 7 | CRDT sync protocol |

---

## Inspiration

> "The Central Idea: the computation kernel IS the product."

QumuloDB draws its design philosophy from **Artemis** (1977) and **Gentia** (1990s) — systems where the project model was the engine, not a layer over a general-purpose database. Artemis A2 had `TIME`, `PERIOD`, `CALENDAR`, `ACTIVITY`, `RESOURCE`, and `COST-ACCOUNT` as first-class language primitives. QumuloDB pursues the same idea in Rust for the 2020s and beyond.

---

## License

Apache 2.0 — see [LICENSE](LICENSE).
