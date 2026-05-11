# QQL — Qumulo Query Language

QQL is standard SQL extended with project management semantics. It is inspired by the Artemis A2 language, where `TIME`, `PERIOD`, `CALENDAR`, `ACTIVITY`, and `RESOURCE` were first-class language primitives — not columns in a table, but concepts baked into the language itself.

QQL speaks standard SQL so that any database developer can use it immediately. The PM extensions are additive, not breaking.

---

## Bitemporal clauses

### `AS OF VALID TIME`

Pins the valid-time axis. Returns facts that were true in the real world at the specified moment.

```sql
SELECT activity_id, name, total_float_hours, is_critical
FROM   qdb_activities
WHERE  project_id = 'PROJ-001'
AS OF VALID TIME '2026-03-01'
ORDER BY total_float_hours ASC;
```

### `AS OF TRANSACTION TIME`

Pins the transaction-time axis. Returns facts as they were recorded in QumuloDB at the specified moment — useful for auditing ("what did the system know on date X?").

```sql
SELECT data_date, cpi, spi, eac
FROM   qdb_evm_series
WHERE  project_id = 'PROJ-001'
AS OF TRANSACTION TIME '2026-01-15T09:00:00';
```

### Fully bitemporal

Both axes can be pinned simultaneously:

```sql
SELECT *
FROM   qdb_activities
WHERE  project_id = 'PROJ-001'
AS OF VALID TIME '2026-03-01'
     TRANSACTION TIME '2026-01-15';
```

---

## Schedule-domain clauses

### `SCHEDULE_PERIOD OVERLAPS`

Matches activities whose planned window overlaps a given period. Accepts ISO dates, quarter notation, or named ranges.

```sql
-- All activities overlapping Q2 2026
SELECT activity_id, name, early_start, early_finish
FROM   qdb_activities
WHERE  project_id = 'PROJ-001'
  AND  SCHEDULE_PERIOD OVERLAPS ('2026-Q2');

-- All activities overlapping an explicit range
SELECT activity_id, name
FROM   qdb_activities
WHERE  SCHEDULE_PERIOD OVERLAPS ('2026-04-01', '2026-06-30');
```

---

## Native CPM columns

Activities expose CPM result columns directly:

| Column | Type | Meaning |
|--------|------|---------|
| `early_start` | timestamp | Earliest possible start |
| `early_finish` | timestamp | Earliest possible finish |
| `late_start` | timestamp | Latest allowable start |
| `late_finish` | timestamp | Latest allowable finish |
| `total_float_hours` | float | Hours until project delay |
| `free_float_hours` | float | Hours until successor delay |
| `is_critical` | bool | `total_float_hours <= 0` |

---

## Native EVM columns

EVM series expose standard metrics:

| Column | Meaning |
|--------|---------|
| `pv`, `ev`, `ac` | Planned Value, Earned Value, Actual Cost |
| `cv`, `sv` | Cost Variance, Schedule Variance |
| `cpi`, `spi` | Cost Performance Index, Schedule Performance Index |
| `eac`, `etc`, `vac` | Estimate at Completion, Estimate to Complete, Variance at Completion |
| `tcpi` | To-Complete Performance Index |
| `percent_complete` | EV / BAC × 100 |

---

## Example queries

```sql
-- Critical path activities
SELECT activity_id, name, early_start, early_finish, total_float_hours
FROM   qdb_activities
WHERE  project_id = 'PROJ-001'
  AND  is_critical = true
ORDER BY early_start;

-- Activities behind schedule (negative SV)
SELECT a.activity_id, a.name, e.sv, e.spi
FROM   qdb_activities a
JOIN   qdb_evm_series e ON a.project_id = e.project_id
WHERE  a.project_id = 'PROJ-001'
  AND  e.spi < 1.0
ORDER BY e.spi ASC;

-- Resource over-allocation
SELECT resource_id, resource_name, period_start, overallocation_hours
FROM   qdb_resource_loads
WHERE  project_id = 'PROJ-001'
  AND  overallocation_hours > 0
ORDER BY period_start;

-- P80 cost forecast across projects
SELECT project_id, p80_cost, p80_duration_hours
FROM   qdb_simulations
WHERE  computed_at > '2026-01-01'
ORDER BY p80_cost DESC;
```

---

## Implementation status

| Feature | Status |
|---------|--------|
| Standard SQL via DataFusion | ✅ Phase 0 scaffold |
| `AS OF VALID TIME` | 🔄 Phase 4 |
| `AS OF TRANSACTION TIME` | 🔄 Phase 4 |
| `SCHEDULE_PERIOD OVERLAPS` | 🔄 Phase 4 |
| Native CPM columns | 🔄 Phase 3 (Arrow table registration) |
| Native EVM columns | 🔄 Phase 3 |
