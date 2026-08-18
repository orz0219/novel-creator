# Backend Architecture Specification

## 1. Architecture Principles

1. **Domain Layer** - Pure business logic, no infrastructure dependencies
2. **Application Layer** - Orchestrates domain operations, CQRS pattern
3. **Infrastructure Layer** - Database, LLM, artifacts, observability
4. **LLM never touches database** - All AI modifications go through Proposal system
5. **Text is derived artifact** - World model is source of truth, not prose

## 2. Domain Model

Core entities:
- **Entity** - Unified world object with version, audit metadata
- **World** - Container for entities and facts
- **Fact** - Objective world truth
- **Relation** - Entity relationships
- **Event** - World events
- **State** - Current world state
- **Narrative** - Story structure (Volume/Arc/Chapter/Scene)
- **Knowledge** - Character knowledge
- **Proposal** - AI change proposals with state machine
- **Job** - Background task management

## 3. Data Ownership

- Domain layer owns types and business rules
- Infrastructure layer owns database operations
- Application layer owns orchestration logic

## 4. Repository Rules

- Domain defines repository traits
- Infrastructure implements DuckDB repositories
- All repositories use Unit of Work for transactions
- Read operations can be concurrent
- Write operations go through Write Queue

## 5. Transaction Rules

- All multi-step operations use Unit of Work
- Transactions are short-lived (no LLM calls inside)
- Rollback on any failure
- Optimistic versioning for conflict detection

## 6. Command / Query Rules

- Commands modify state (POST)
- Queries read state (GET)
- Commands go through Write Queue
- Queries can be concurrent

## 7. Proposal Rules

- AI cannot directly modify world
- All changes go through ProposedChange
- State machine: Draft -> Validating -> Valid -> Approved -> Committed
- Schema validation -> Domain validation -> Canon validation -> Conflict detection

## 8. State Management

- Canonical Data (entities, facts, relations)
- Immutable History (events, state_changes)
- Projection (current_state, current_knowledge)
- Both history and current state maintained

## 9. Job Lifecycle

- State machine: Pending -> Running -> WaitingInput -> Completed
- Exceptions: Failed, Cancelled, Timeout
- Jobs are recoverable after server restart
- Job progress tracking

## 10. Generation Lifecycle

- Separate from Job (one Job can have multiple Generations)
- Full versioning (skill, prompt, schema, context_policy)
- Retry support with model switching
- Complete provenance tracking

## 11. Context Lifecycle

- Context Snapshots are immutable after creation
- L0-L6 layer system
- Token budget management
- Context traces for explainability

## 12. Concurrency Rules

- Single Rust process manages all DuckDB access
- Write Queue serializes mutations
- Read operations can be concurrent
- Optimistic versioning prevents conflicts

## 13. Error Model

- DomainError: Business logic errors
- ApplicationError: Service layer errors
- InfrastructureError: Database, LLM, storage errors
- Unified NovelError type

## 14. Event Model

- Domain Events for system internal tracking
- Event Dispatcher for routing
- Audit Log for persistence
- WebSocket for real-time UI updates

## 15. Artifact Model

- Large text stored outside DuckDB
- Content hash for integrity
- Metadata in DuckDB, content in filesystem
- Support for future object storage

## 16. Migration Rules

- Version-tracked migrations
- Backward compatibility required
- Idempotent migrations

## 17. Testing Strategy

- Unit tests for domain logic
- Integration tests for repositories
- E2E tests for full workflows
- Concurrency tests for write queue
- Rollback tests for transactions

## 18. DuckDB Constraints

- Single-process writes
- Short transactions
- No long-running transactions
- Read operations can be concurrent
- Write conflicts need retry logic
