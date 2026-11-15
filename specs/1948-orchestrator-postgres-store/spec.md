# Feature Specification: PostgreSQL store for the orchestrator (SQLite for local mode)

**Feature Branch**: `claude/project-thread-ns6vcg`

**Created**: 2026-11-15

**Status**: Draft

**Input**: Issue #1948 "feat: PostgreSQL store for the orchestrator (sqlx, migrations, store traits;
SQLite for local mode)", part of #1722, milestone M3, depends on #1724 (`RegistryStore` trait).
Covers FR-042 and FR-046 of `specs/1722-rn-client-k8s/spec.md`. Plan: `specs/1722-rn-client-k8s/plan.md`
*Storage: PostgreSQL* and *Toward multi-user*.

## Clarifications

- Local mode on one machine uses SQLite; network deployments use PostgreSQL (jaro, 2026-11-15).
- Every daemon, project and session record carries an owner and a tenant from the start (jaro).
- Dependency: the constitution amendment that allows PostgreSQL for network deployments (#1949) is
  not merged when this spec is written, and the constitution text still says local-only storage.
  Nothing in this feature's behaviour waits on it; #1949 must merge before a network deployment
  ships, not before this code merges.
- Linux only: jaro dropped macOS and Windows on 2026-11-14 (#1897). That decision stands whatever
  the constitution text says until #1949 updates it.

### Session 2026-11-15

- Q: When a `[database]` table is present on a loopback-only bind, which backend runs? → A:
  PostgreSQL. The table's presence selects PostgreSQL on any bind; SQLite is chosen only when the
  table is absent on a loopback bind (FR-008, FR-009). _(agent-resolved:
  specs/1722-rn-client-k8s/plan.md#Storage: PostgreSQL)_
- Q: Does moving an existing SQLite local-mode database to PostgreSQL belong here? → A: No. Only
  the one-shot `registry.json` import exists (FR-010); a SQLite-to-PostgreSQL data move is out of
  scope. _(agent-resolved: specs/1722-rn-client-k8s/plan.md#Storage: PostgreSQL)_
- Q: Which `owner_user_id` do records get before any user exists (enrolled daemons, imported
  `registry.json` records)? → A: Deferred to the plan, which must pick a value that satisfies
  FR-007 and FR-010 together (v1 has one tenant and one owner). _(agent-resolved:
  specs/1722-rn-client-k8s/plan.md#Toward multi-user)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Orchestrator state survives a restart (Priority: P1)

An operator runs the orchestrator against PostgreSQL (network) or a SQLite file (local mode). They
register daemons, create users and generate audit entries, then restart the orchestrator. Everything
is still there.

**Why this priority**: This is the feature's reason to exist; every later M3 issue (sign-in, roles,
JSON API) reads and writes through it.

**Independent Test**: Against each backend, write registry records, a user and an audit entry
through the three stores, drop and reopen the store, and read the same data back.

**Acceptance Scenarios**:

1. **Given** a PostgreSQL database with registered daemons, a user and audit entries, **When** the
   orchestrator restarts, **Then** the registry, the user and the audit entries are all present.
2. **Given** a local-mode SQLite file with the same data, **When** the orchestrator restarts,
   **Then** the same data is present.
3. **Given** the in-memory implementation, **When** the same store test suite runs, **Then** it
   passes too, so the three implementations behave the same through the traits.

---

### User Story 2 - A newer schema is refused (Priority: P1)

An operator starts an older orchestrator build against a database that a newer build has already
migrated.

**Why this priority**: Without it a rollback silently corrupts or misreads state.

**Independent Test**: Migrate a database, record a schema version above the build's newest, start
the store, and read the error.

**Acceptance Scenarios**:

1. **Given** a database whose schema version is newer than the build knows, **When** the
   orchestrator starts, **Then** it refuses, the error names both the database's version and the
   build's newest version, and it changes nothing in the database.
2. **Given** an empty or older database, **When** the orchestrator starts, **Then** it applies the
   missing migrations, in order, and only forward.

---

### User Story 3 - No private key reaches the database (Priority: P1)

A reviewer or an auditor checks that a database dump holds no private key.

**Why this priority**: Secrets (CA key, signing keys) stay in files or Kubernetes Secrets (FR-042);
a key in a backup would defeat that.

**Independent Test**: Store every record kind that touches certificates or tokens, then scan every
column of every table for private-key material.

**Acceptance Scenarios**:

1. **Given** the stores hold certificate serials, revocations, token hashes and public data, **When**
   every table is dumped, **Then** no PEM private-key block and no column named for a key exists.
2. **Given** a caller passes private-key material to a store method, **When** it is saved, **Then**
   the secret-bearing fields (tokens, serials) accept only a hash or hex serial, and the column scan
   finds any key material that reached a free-text field.

---

### User Story 4 - Connect to a database from config (Priority: P2)

An operator names the database in the orchestrator config: `database.url_file` (a file holding the
URL with its password) and `database.ca_file` (TLS to the server).

**Why this priority**: Needed to deploy, but a test setup can pass a URL directly.

**Independent Test**: `config check` on files with and without these keys.

**Acceptance Scenarios**:

1. **Given** `database.url_file` pointing at a readable file, **When** the config loads, **Then**
   the URL is read from the file and never printed; only the path and its state show in the
   resolved view.
2. **Given** a missing or empty `url_file`, or a `ca_file` that does not exist, **When** `config
   check` runs, **Then** it fails naming the key and the path.
3. **Given** no `[database]` table and the orchestrator bound to loopback only, **When** it starts,
   **Then** it uses a SQLite file under the data directory.
4. **Given** no `[database]` table and a non-loopback bind address, **When** the orchestrator starts
   or `config check` runs, **Then** it fails, naming the missing `database.url_file` key. (The
   orchestrator has no network bind yet: this feature ships the rule and its unit test, #1732 wires
   it end to end.)

---

### User Story 5 - Import an existing `registry.json` once (Priority: P2)

An operator who ran #1724's JSON registry moves to the database.

**Why this priority**: Without it the move loses the registry, but few installs exist yet.

**Independent Test**: Import a `registry.json` into an empty store; import again.

**Acceptance Scenarios**:

1. **Given** a `registry.json` not yet imported and an empty database registry (either backend),
   **When** the orchestrator starts, **Then** it imports every record automatically and renames the
   file with a suffix that marks it as imported.
2. **Given** the import already ran, **When** the orchestrator starts again, **Then** nothing is
   imported and nothing changes.
3. **Given** a corrupt `registry.json`, **When** the orchestrator starts, **Then** the start fails
   naming the file and imports nothing.
4. **Given** an un-imported `registry.json` and a database registry that already has rows, **When**
   the orchestrator starts, **Then** it refuses, naming the file and the row count, and imports
   nothing.

---

### User Story 6 - Ownership columns from the start (Priority: P3)

Every stored row carries a tenant, and every daemon, project and session row carries an owner, so
multi-user later needs no data migration (FR-046).

**Why this priority**: Cheap now, a data migration later.

**Independent Test**: Inspect the schema and round-trip records with distinct tenants and owners.

**Acceptance Scenarios**:

1. **Given** the migrated schema, **When** its tables are listed, **Then** every table that holds
   orchestrator data has a `tenant_id` (the migration bookkeeping table is the one exception) and
   the daemon, project and session tables have an `owner_user_id`.
2. **Given** auth and audit data of two tenants in one database, **When** one tenant's store reads,
   **Then** it sees only its own rows (registry records carry their tenant and load together).

### Edge Cases

- Empty database: first start migrates from nothing; an empty registry loads as empty, not an error.
- Database unreachable or credentials wrong: start fails naming the host and cause, never the
  password; a transient loss after start returns a store error and the orchestrator keeps running.
- Concurrency (Principle II): two orchestrators migrating one database at once; exactly one applies
  each migration and the other waits and then sees it applied. Concurrent saves of the registry do
  not lose a record the other writer did not touch (the traits offer per-record upsert and remove,
  FR-015).
- A migration fails half way: the database stays at the previous version (each migration runs in a
  transaction) and the start fails.
- A migration file edited after release: the start refuses, naming the migration, because its
  checksum differs from the recorded one.
- The audit log is append-only: the stores offer no update or delete of an audit entry, and the
  PostgreSQL role needs no such grant.
- SQLite file on a path that is not writable, or on a network filesystem: start fails with the path.
- Cross-platform (Principle VI): the orchestrator is Linux only (#1897); the SQLite file is
  owner-only (0600).
- `url_file` with a trailing newline or whitespace: trimmed (FR-008).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The orchestrator MUST keep its state behind three traits, `RegistryStore`,
  `AuthStore` and `AuditStore`, each with an in-memory implementation (tests), a PostgreSQL
  implementation and a SQLite implementation (local mode). The existing `RegistryStore` contract
  from #1724 stays valid for current callers.
- **FR-002**: Registry records, users, identities, invites, the bootstrap code hash, refresh-token
  hashes, certificate serials and revocations, enrollments and audit entries MUST survive an
  orchestrator restart on PostgreSQL and on SQLite.
- **FR-003**: Schema migrations MUST apply on start, run forward only, and each run all-or-nothing.
  When two orchestrators start on one database, exactly one applies each migration and the other
  waits, then sees it applied.
- **FR-004**: The orchestrator MUST refuse a database whose schema version is newer than its newest
  migration; the error MUST name the database's version and the build's version and MUST leave the
  database unchanged.
- **FR-005**: The orchestrator MUST refuse a database whose recorded checksum for an applied
  migration differs from the shipped one.
- **FR-006**: No private key MUST ever be written to the database: the store types hold only
  serials, hashes and public data, and a test MUST scan every column after exercising every store.
- **FR-007**: Every table that holds orchestrator data (all but the migration bookkeeping table)
  MUST have a `tenant_id`; every daemon, project and session table MUST have
  an `owner_user_id`. In v1 the single tenant and its owner fill them; `AuthStore` and `AuditStore` reads MUST
  filter by tenant. `RegistryStore` loads every tenant's records, because each record names its
  own tenant (plan.md, *Tenant scope*).
- **FR-008**: The orchestrator config MUST accept `database.url_file` (path to a file holding the
  PostgreSQL URL), `database.ca_file` (CA for TLS to the server) and a pool size, name secrets by
  path only, and show no URL or password in any output. The URL read from the file is trimmed, and
  a scheme other than `postgres` or `postgresql` is refused by `config check`.
- **FR-009**: With no `[database]` table and a loopback-only bind, the orchestrator MUST use SQLite
  in the data directory (owner-only file) and MUST NOT need a database server. With a non-loopback
  bind and no `[database]` table it MUST refuse to start, and `config check` MUST fail, naming the
  missing key (the rule lands here; the bind key that triggers it lands with #1732).
- **FR-010**: On start, when an un-imported `registry.json` exists, the orchestrator MUST import it
  into an empty database registry (either backend) and mark the file as imported; it MUST do
  nothing on later starts. A corrupt file, or a database registry that already has rows, MUST fail
  the start naming the file (and the row count) and import nothing.
- **FR-011**: Audit entries MUST be append-only through the trait: no update or delete operation
  exists, and the PostgreSQL migration MUST NOT grant one to the orchestrator's role.
- **FR-012**: The store tests MUST run against the in-memory, SQLite and PostgreSQL implementations
  from one shared suite, and CI MUST run the PostgreSQL ones against a Postgres service container.
- **FR-013**: CI MUST test the migration chain from every released schema version to the newest:
  a database built at version N by the shipped migrations upgrades to the current one with its data
  intact.
- **FR-014**: A slow database call MUST NOT stall daemon links: heartbeats from other daemons are
  still handled within the 9 s link liveness deadline of #1724 while a store call is outstanding.
- **FR-015**: The registry store MUST offer per-record upsert and remove, besides the existing
  whole-list `save`, which stays for current callers.
- **FR-016**: An unreachable database at start MUST fail the start with the host and cause, never
  the password. A database lost after start MUST make store calls return an error while the
  orchestrator keeps running and retries on the next call.

### Key Entities

- **Tenant**: the isolation key on every row; one tenant in v1.
- **Daemon record**: a registered daemon (as in #1724), with tenant and owner.
- **Project / session record**: rows that name a project or a session of a daemon, with tenant and
  owner (schema only here; their writers come with later issues).
- **User, identity, invite, bootstrap hash, refresh-token hash**: the `AuthStore` data; hashes only.
- **Certificate serial, revocation, enrollment**: public certificate facts; never a key.
- **Audit entry**: an append-only event with tenant, actor, action, target and time.
- **Schema version**: what the database has applied, with a checksum per migration.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: After a restart, 100% of the registry, user and audit rows written before it are
  readable, in each deployment mode (network and local).
- **SC-002**: Starting against a newer schema fails 100% of the time, with both versions in the
  message, and no migration row, table or data row is added, changed or removed by the attempt (the SQLite file
  is byte-identical).
- **SC-003**: A scan of every column of every table after the full store suite finds zero
  private-key blocks.
- **SC-004**: The one shared store suite passes for every implementation, in CI for each
  deployment mode, and the migration chain test passes from every released schema.
- **SC-005**: Local mode starts with no external service and no configuration beyond the data
  directory.

## Assumptions

- The access layer (`sqlx`, as the 1722 plan decided), the migrations directory
  (`crates/micold-daemon/migrations/`), threading and the exact trait signatures (sync as in #1724,
  or async) belong to the plan unit.
- "Released schema" means each schema version a shipped orchestrator build could have written;
  before this feature none was released, so the chain starts at this feature's first version, and
  the test enumerates the migration files so each later release adds itself.
- Project and session tables carry the ownership columns now; the code that fills them comes with
  the issues that create projects and sessions.
- Linux only; Kubernetes chart work (`database.existingSecret`) is a later issue.

## Out of Scope

- Sign-in, roles, invites flow, the JSON API (other M3 issues); this feature stores their data.
- Per-user authorization and multi-user behaviour (N17); only the columns exist.
- The Postgres `LinkDirectory`, replica forwarding, retention partition drops, backups.
- Constitution amendment (#1949); Helm chart changes.
- Encrypting the SQLite file at rest.
- Moving a SQLite local-mode database into PostgreSQL (only the `registry.json` import exists).
