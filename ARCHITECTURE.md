# Novel Creator Architecture

## 1. Architecture Goal

Novel Creator is a stateful AI-assisted novel creation system.

The system is not primarily an LLM wrapper and not primarily a CRUD application.

Its core responsibility is:

```text
World State
    ↓
Context
    ↓
Generation
    ↓
Proposal
    ↓
Validation
    ↓
Commit
    ↓
World State
```

The system must preserve a stable canonical story state while allowing AI to generate drafts and propose state changes.

---

# 2. Core Principles

## Principle 1: Canonical State Is the Source of Truth

Novel text is not the source of truth.

The canonical state consists of structured domain data such as:

* entities
* characters
* factions
* relations
* facts
* story events
* character knowledge
* narrative state
* timeline
* causal relationships
* author-confirmed constraints

Draft text is a derived artifact.

```text
Canonical State + Narrative Intent + Context
    ↓
Draft
```

A draft must never silently become canonical state.

---

## Principle 2: AI Cannot Directly Modify Canonical State

AI output must never directly mutate world state.

The write path is:

```text
LLM
    ↓
Generation Output
    ↓
ProposedChange
    ↓
Validation
    ↓
Approved
    ↓
Commit
    ↓
Canonical State
```

Only the commit boundary can mutate canonical state.

---

## Principle 3: Context Is a Read Policy

The AI must not receive the entire database.

Context construction determines:

* what information is visible
* why the information is relevant
* how much information is included
* which information has higher priority
* which character knows the information
* which narrative information has already been revealed

The Context subsystem is therefore a read-policy layer, not a database layer.

---

## Principle 4: World Knowledge and Character Knowledge Are Different

The following concepts must remain separate:

```text
World Truth
Narrative Revelation
Character Knowledge
Reader Knowledge
```

Example:

```text
World:
    The antagonist is secretly the king.

Reader:
    The reader has seen hints but does not know for certain.

Character A:
    Character A believes the antagonist is trustworthy.

Character B:
    Character B knows the antagonist is the king.
```

Context construction must respect these visibility boundaries.

---

## Principle 5: History Is Append-Only

Historical facts must not be silently overwritten.

State evolution follows:

```text
Event
    ↓
StateChange
    ↓
Current Projection
```

Current state exists for efficient reads.

Historical records exist for:

* audit
* rollback
* debugging
* reconstruction
* causal analysis

The system does not require full event-sourcing replay for every read.

The intended model is:

```text
Append-only History
        +
Current State Projection
```

---

## Principle 6: Story Events and System Events Are Different

A Story Event describes what happens inside the novel.

Example:

```text
The protagonist defeats the demon king.
```

A Domain Event describes what happens inside the software system.

Example:

```text
EntityStateChanged
```

These concepts must never share the same abstraction merely because both are called "events".

```text
Story Event != Domain Event
```

---

# 3. System Architecture

The system is divided into five logical responsibilities:

```text
                    Application
                        │
        ┌───────────────┼────────────────┐
        │               │                │
        ▼               ▼                ▼
      World         Generation        Query/Read
        │               │                │
        │          Context Builder      │
        │               │                │
        │           Skill / LLM          │
        │               │                │
        │           Proposal             │
        │               │                │
        └───────────────┼────────────────┘
                        ▼
                    Validation
                        │
                        ▼
                      Commit
                        │
                        ▼
                  Domain Event
                        │
                        ▼
                Current Projection
                        │
                        ▼
                   PostgreSQL
```

The central lifecycle is:

```text
READ
 ↓
CONTEXT
 ↓
GENERATE
 ↓
PROPOSE
 ↓
VALIDATE
 ↓
COMMIT
 ↓
PROJECT
 ↓
READ
```

---

# 4. Crate Responsibilities

## 4.1 domain

`domain` contains the canonical business model and business invariants.

It must not depend on:

* PostgreSQL
* SQLx
* HTTP
* Axum
* LLM providers
* prompt implementation
* Context implementation
* database repositories
* runtime implementation

The domain contains:

```text
Project
World
Entity
Character
Faction
Narrative
Storyline
Knowledge
State
StoryEvent
StateChange
ProposedChange
Validation
Ledger
Repository Traits
```

The domain answers:

```text
What is true?
What state exists?
What changes are legal?
What constitutes a valid proposal?
```

The domain does not answer:

```text
How do we query PostgreSQL?
How do we call an LLM?
How do we build a prompt?
How do we execute a job?
```

---

# 5. Application Layer

`application` contains use cases.

It coordinates domain operations and external ports.

Examples:

```text
CreateProject
CreateWorld
CreateEntity
PlanNarrative
GenerateScene
CreateProposal
ValidateProposal
CommitProposal
QueryTimeline
QueryStoryline
```

Application is responsible for:

* use-case orchestration
* transaction boundaries
* authorization/project isolation
* coordinating repositories
* coordinating generation/runtime capabilities
* returning application-level results

Application must not contain:

* SQL queries
* PostgreSQL-specific code
* Prompt construction
* LLM provider implementation
* low-level retrieval logic

The intended dependency direction is:

```text
Application
    ↓
Domain Interfaces
    ↑
Infrastructure Implementations
```

---

# 6. Runtime Layer

`runtime` contains execution-time AI capabilities.

Runtime is not the database layer.

Runtime responsibilities include:

```text
Context Construction
Validation Execution
Extraction
Generation Runtime
State Commit Coordination
```

Runtime must not own PostgreSQL access.

The desired dependency is:

```text
runtime
    ↓
domain
```

Database access is provided through ports/repositories supplied by the application composition root.

---

# 7. Context System

Context construction is a dedicated subsystem.

The architecture is:

```text
Repository / Retrieval
        ↓
RetrievalResult
        ↓
Visibility Policy
        ↓
Relevance Ranking
        ↓
Token Budget
        ↓
Context Builder
        ↓
Context Snapshot
```

The Context subsystem must not directly contain SQL query implementations.

Retrieval answers:

```text
What data is relevant?
```

Context Policy answers:

```text
What should the model see?
```

These are separate responsibilities.

---

## 7.1 Retrieval

Retrieval may use:

```text
Structured Query
Graph Traversal
Temporal Query
Semantic Search
```

Structured retrieval is the primary mechanism for canonical world information.

Semantic search is supplementary.

---

## 7.2 Visibility

Visibility determines whether information is visible to:

```text
Writer
Character
Reader
Planner
Validator
```

Visibility must account for:

```text
World Truth
Narrative Revelation
Character Knowledge
Reader Knowledge
```

---

## 7.3 Context Snapshot

Every LLM generation should be reproducible from a recorded context snapshot.

A context snapshot should record:

```text
project_id
scene_id
policy_version
token_budget
selected information
content hash
```

The snapshot is evidence of what the model was allowed to see.

---

# 8. Generation System

Generation is capability-oriented rather than workflow-oriented.

Do not hard-code one universal pipeline.

Capabilities include:

```text
Scene Planning
Scene Writing
Story Planning
Knowledge Extraction
State Change Extraction
Validation
Revision
```

A particular use case may invoke any subset.

For example:

```text
Write Scene:

Context
 ↓
Scene Writer
 ↓
Draft
```

While a state-changing operation may use:

```text
Context
 ↓
Generator
 ↓
ProposedChange
 ↓
Validator
 ↓
Commit
```

The system must not require every operation to pass through:

```text
Planner
 → Writer
 → Extractor
 → Validator
 → RevisionPlanner
```

That is one possible workflow, not a system invariant.

---

# 9. Proposal System

AI-generated state changes are represented as `ProposedChange`.

A proposal contains:

```text
project
task/generation
change type
target
description
payload
status
```

Lifecycle:

```text
Draft
 ↓
Validated
 ↓
Approved / Rejected
 ↓
Committed
```

The proposal is an explicit boundary between probabilistic AI output and deterministic application state.

---

# 10. Validation

Validation must happen before commit.

Validation checks:

```text
Schema validity
Entity existence
Project isolation
State consistency
Timeline consistency
Relation consistency
Character knowledge constraints
Narrative constraints
Canon constraints
```

Validation is deterministic whenever possible.

LLM-based validation may provide additional semantic checks, but it must not replace hard domain invariants.

---

# 11. Commit

Commit is the only canonical write boundary.

The commit operation must:

1. Reload authoritative proposals.
2. Verify proposal status.
3. Verify project isolation.
4. Verify target existence.
5. Check optimistic concurrency/version constraints.
6. Create domain events.
7. Create state changes.
8. Update current projections.
9. Commit the database transaction atomically.

Failure of any step must roll back the complete transaction.

```text
Approved Proposal
        ↓
Authoritative Reload
        ↓
Invariant Checks
        ↓
Domain Event
        ↓
State Change
        ↓
Current Projection
        ↓
COMMIT
```

---

# 12. Database Architecture

The current persistence implementation is:

```text
PostgreSQL
+
SQLx
```

The database layer is an infrastructure implementation.

```text
Domain Repository Trait
          ↑
          │
PostgreSQL Repository
          │
        SQLx
          │
     PostgreSQL
```

Application and Runtime must not depend on PostgreSQL-specific types such as:

```text
PgPool
sqlx::Query
Postgres-specific row types
```

outside the infrastructure/database boundary.

---

# 13. Database Concepts

The persistence model contains several different concepts.

## Story Event

An event inside the novel.

```text
event
```

## State Change

A change caused by a story event.

```text
state_change
```

## Current State

The current read projection.

```text
current_state
```

## Domain Event

An event describing a change inside the application domain.

```text
domain_event
```

## Audit Log

A historical record describing system-level modifications.

```text
audit_log
```

These concepts must remain separate.

---

# 14. Current State and History

The system uses:

```text
History
    +
Projection
```

rather than full event-sourcing.

Historical state changes are append-only.

Current state is optimized for reads.

Therefore:

```text
state_change
    ↓
current_state
```

does not imply that every query must replay all historical events.

Snapshots are optional optimization/recovery artifacts rather than the primary source of truth.

---

# 15. Narrative Model

Narrative structure is domain data.

It includes concepts such as:

```text
Volume
Arc
Storyline
Scene
Narrative Thread
Foreshadowing
Narrative Budget
```

Narrative planning does not directly modify canonical world state.

Narrative planning produces intent.

```text
Narrative Intent
    ↓
Generation
    ↓
Proposal
    ↓
Validation
    ↓
Commit
```

---

# 16. Character Knowledge

Character knowledge is a separate state dimension.

It must not be inferred directly from world state at generation time.

The system may know:

```text
World:
    Secret X exists.
```

while:

```text
Character A:
    Does not know X.
```

and:

```text
Character B:
    Knows X.
```

Context construction must preserve this distinction.

---

# 17. Runtime and Database Boundary

The following architecture is forbidden:

```text
Runtime
   ↓
PgPool
   ↓
SQLx
```

The following is preferred:

```text
Runtime
   ↓
Repository / Retrieval Port
   ↑
PostgreSQL Implementation
```

Likewise:

```text
ContextEngine
   ↓
SQL query
```

is forbidden.

Instead:

```text
ContextEngine
   ↓
Retriever
   ↓
Repository
```

This keeps Context responsible for information policy rather than persistence.

---

# 18. API / Application Host

`narrative-engine` is the application host.

Its responsibilities are:

```text
Process startup
Dependency construction
HTTP API
WebSocket/API transport
Application wiring
Configuration
```

It must not become another business-logic layer.

The dependency direction is:

```text
narrative-engine
        ↓
application
        ↓
runtime
        ↓
domain
```

Infrastructure/database implementations are wired at the composition root.

---

# 19. Infrastructure

Infrastructure contains implementations of external concerns:

```text
PostgreSQL
LLM Providers
Artifact Storage
Observability
External Services
```

Infrastructure should implement interfaces required by Domain/Application/Runtime.

It should not define business rules.

---

# 20. Recommended Dependency Graph

The target dependency graph is:

```text
                         narrative-engine
                                │
                                ▼
                           application
                         ┌──────┴──────┐
                         ▼             ▼
                      runtime        domain
                         │             ▲
                         └──────┬──────┘
                                │
                       domain interfaces
                                ▲
                                │
                         infrastructure
                                │
                         ┌──────┴──────┐
                         ▼             ▼
                     PostgreSQL       LLM
```

The important rule is:

```text
Domain
    ↓
must never depend on infrastructure.
```

And:

```text
Runtime
    ↓
must not directly depend on PostgreSQL.
```

---

# 21. What Must Be Avoided

Do not introduce new abstractions merely to preserve the existing crate structure.

Avoid:

```text
WorldEngine
WorldManager
WorldService
WorldRuntime
WorldRepository
WorldContext
WorldCoordinator
```

when they all perform overlapping responsibilities.

Avoid giant modules that simultaneously:

```text
Query DB
Transform DB rows
Apply business rules
Build context
Call LLM
Validate output
Commit state
```

Each stage should have one clear responsibility.

---

# 22. Core Architectural Boundary

The most important boundary in the system is:

```text
Probabilistic World
        │
        │ AI Generation
        ▼
    Proposal
        │
        │ Deterministic Validation
        ▼
 Canonical World
```

Everything before Proposal is allowed to be probabilistic.

Everything after Commit must obey deterministic domain rules.

---

# 23. Canonical Workflow

The recommended canonical workflow is:

```text
1. Receive user intent
        ↓
2. Application creates a use-case
        ↓
3. Retrieve relevant canonical state
        ↓
4. Build Context Snapshot
        ↓
5. Execute generation capability
        ↓
6. Produce Draft and/or ProposedChange
        ↓
7. Validate
        ↓
8. Ask for approval when required
        ↓
9. Commit approved changes
        ↓
10. Append Domain Event / State Change
        ↓
11. Update Current State Projection
        ↓
12. Persist generation/artifact metadata
```

Not every generation operation needs every step.

---

# 24. Refactoring Priorities

The implementation should converge toward this order:

```text
P0
Remove architecture/documentation contradictions.

P1
Remove direct PostgreSQL dependencies from Runtime.

P2
Move database-specific retrieval out of ContextEngine.

P3
Make Application depend on repository/use-case ports rather than concrete DB implementation.

P4
Reduce Domain to canonical business concepts and invariants.

P5
Split ContextEngine into:
    Retrieval
    Visibility
    Ranking
    Budget
    ContextBuilder

P6
Keep Proposal → Validate → Commit as the single canonical mutation path.

P7
Reduce narrative-engine to application host/API composition.

P8
Only introduce additional abstractions when a real use case requires them.
```

---

# 25. Target Architecture

The final architecture should remain intentionally small:

```text
                        USER
                         │
                         ▼
                  Application API
                         │
                         ▼
                    Application
                         │
             ┌───────────┴───────────┐
             │                       │
             ▼                       ▼
           Domain                 Runtime
             │                       │
             │                 ┌─────┴─────┐
             │                 ▼           ▼
             │             Retrieval    Context
             │                 │           │
             │                 └─────┬─────┘
             │                       ▼
             │                      LLM
             │                       │
             │                       ▼
             │                    Proposal
             │                       │
             └───────────────────────▼
                                  Validate
                                     │
                                     ▼
                                   Commit
                                     │
                                     ▼
                               Domain Event
                                     │
                                     ▼
                              Current State
                                     │
                                     ▼
                                PostgreSQL
```

The system's core invariant is:

```text
AI may propose.
Domain decides.
Commit persists.
```

Everything else is implementation detail.
