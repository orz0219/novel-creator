# Novel Engine Architecture

## 5 Frozen Principles (System Constitution)

### Principle 1: World Truth ≠ Novel Text
Text is just an expression of world and narrative state. The world model is the source of truth, not the prose.

### Principle 2: AI Cannot Directly Modify Canon
AI can only propose `ProposedChange`. All changes must go through Validator → Commit. Never allow AI to write directly to the world database.

### Principle 3: Context Determines What AI Can See
Don't dump the entire database into the model. The Context Engine constructs the minimum sufficient context for each skill.

### Principle 4: What Characters Know ≠ What Exists in the World
Must maintain 4 separate state dimensions:
- **World State**: What actually happened (objective truth)
- **Narrative State**: What has been revealed to readers
- **Character State**: What each character knows/feels
- **Reader State**: What the reader knows

### Principle 5: History Can Never Be Overwritten
State changes follow: `Event → StateChange → Projection`
Never use `UPDATE old_value` to overwrite history.

---

## Derived Artifact Principle

Novel text is NOT the world truth. It is a **derived artifact**:

```
World Model + Narrative Plan + Context → Draft
```

This makes chapter revision trivial - just regenerate from the world model.

---

## Source Priority System

When conflicting information exists, the system resolves by priority:

```
Canon Rule > World State > Author Confirmed Data > Structured Design > Previous Draft > AI Generated Text
```

AI-generated text is the LOWEST priority. This prevents AI from polluting the world database.

---

## Architecture Overview

```
                    ┌───────────────────┐
                    │  Canon Constitution│
                    │  (RULE-0~3)       │
                    └─────────┬─────────┘
                              │
                     ┌────────▼────────┐
                     │   WORLD ENGINE   │
                     │                  │
                     │ Entity           │
                     │ Fact (Certainty) │
                     │ Rule             │
                     │ Relation         │
                     │ Event            │
                     │ State            │
                     │ Timeline         │
                     │ Causal Graph     │
                     └────────┬─────────┘
                              │
             ┌────────────────┼────────────────┐
             │                │                │
             ▼                ▼                ▼
       Character          Narrative         Knowledge
         Mind               Plan             Model
         ├─ Knowledge       ├─ Volume       ├─ Revelation
         ├─ Belief          ├─ Arc          ├─ Reader Knowledge
         ├─ Memory          ├─ Storyline    ├─ Narrative State
         ├─ Goal            ├─ Scene         └─ Visibility
         ├─ Fear            │
         └─ Emotion         │
                            │
             └────────────────┼────────────────┘
                              ▼
                     ┌────────────────┐
                     │ CONTEXT ENGINE │
                     │                │
                     │ Structured     │
                     │ Graph          │
                     │ Timeline       │
                     │ Semantic       │
                     │ Visibility     │
                     │ Ranking        │
                     │ Compression    │
                     └────────┬───────┘
                              ▼
                       Context Snapshot
                       + Context Trace
                              │
                              ▼
                           SKILL
                           ├─ Planner
                           ├─ Writer
                           ├─ Extractor
                           ├─ Validator
                           └─ RevisionPlanner
                              │
                              ▼
                             LLM
                              │
                 ┌────────────┴────────────┐
                 ▼                         ▼
               Draft                 Proposed Changes
                 │                         │
                 ▼                         ▼
             Validator                State Validator
                 │                         │
                 └────────────┬────────────┘
                              ▼
                       Narrative Ledger
                       + Decision Trace
                              │
                              ▼
                         World State
                         + State Snapshots (Rollback)
```

---

## Workflow: Planner → Writer → Extractor → Validator → RevisionPlanner

```
Planner (ScenePlanner/VolumePlanner/ArcPlanner)
    ↓
Scene Plan
    ↓
Writer (SceneWriter)
    ↓
Draft
    ↓
Extractor (KnowledgeExtractor/StateChangeExtractor)
    ↓
Validator (ContinuityValidator)
    ↓
Issues (if any)
    ↓
RevisionPlanner
    ↓
Revision Plan
    ↓
Writer (polish)
    ↓
Final Draft
    ↓
Commit → Narrative Ledger → World State
```

---

## Multi-Level Memory Hierarchy

```
Scene Ledger (detailed)
    ↓
Chapter Summary (medium)
    ↓
Arc Summary (summary)
    ↓
Volume Summary (compressed)
    ↓
Global Story State (highest compression)
```

---

## Retrieval System

```
Structured Retrieval (SQL queries - primary)
    +
Graph Retrieval (relation traversal)
    +
Temporal Retrieval (time-based queries)
    +
Semantic Retrieval (vector search - supplementary only)
    ↓
Merged Results
```

**Key principle**: Don't rely on RAG as primary. Novel world information is highly structured - SQL/Graph queries are far more reliable than vector search.
