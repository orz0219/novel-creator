> ⚠️ **本文档已归档（2026-08-21）**：内容与真实数据库 schema 严重漂移（真实库 92 张表，见 `crates/db/migrations/001–017`）。
> 请勿以本文档为对照基准。Schema 的唯一事实来源是 `crates/db/migrations/` 与 `crates/db/src/schema.rs`。
> 背景见 `FRONTEND_AUDIT_REPORT.md` 第一节。

# Database Schema Specification

## Tables

### project
| Column | Type | Constraints |
|--------|------|-------------|
| id | UUID | PRIMARY KEY |
| name | VARCHAR | NOT NULL |
| description | TEXT | |
| status | VARCHAR | NOT NULL DEFAULT 'active' |
| version | INTEGER | NOT NULL DEFAULT 1 |
| created_at | TIMESTAMP | NOT NULL |
| updated_at | TIMESTAMP | NOT NULL |

### world
| Column | Type | Constraints |
|--------|------|-------------|
| id | UUID | PRIMARY KEY |
| project_id | UUID | NOT NULL REFERENCES project(id) |
| name | VARCHAR | NOT NULL |
| description | TEXT | |
| rules | JSON | |
| is_main | BOOLEAN | DEFAULT 0 |
| version | INTEGER | NOT NULL DEFAULT 1 |
| created_at | TIMESTAMP | NOT NULL |
| updated_at | TIMESTAMP | NOT NULL |

### entity
| Column | Type | Constraints |
|--------|------|-------------|
| id | UUID | PRIMARY KEY |
| project_id | UUID | NOT NULL REFERENCES project(id) |
| world_id | UUID | NOT NULL |
| entity_type_id | UUID | NOT NULL |
| name | VARCHAR | NOT NULL |
| summary | TEXT | |
| description | TEXT | |
| attributes | JSON | |
| version | INTEGER | NOT NULL DEFAULT 1 |
| created_by | VARCHAR | NOT NULL DEFAULT 'system' |
| updated_by | VARCHAR | |
| source_generation_id | UUID | |
| created_at | TIMESTAMP | NOT NULL |
| updated_at | TIMESTAMP | NOT NULL |

### relation
| Column | Type | Constraints |
|--------|------|-------------|
| id | UUID | PRIMARY KEY |
| project_id | UUID | NOT NULL |
| source_entity_id | UUID | NOT NULL |
| target_entity_id | UUID | NOT NULL |
| relation_type | VARCHAR | NOT NULL |
| description | TEXT | |
| attributes | JSON | |
| valid_from | VARCHAR | |
| valid_until | VARCHAR | |
| version | INTEGER | NOT NULL DEFAULT 1 |
| created_at | TIMESTAMP | NOT NULL |
| updated_at | TIMESTAMP | NOT NULL |

### fact
| Column | Type | Constraints |
|--------|------|-------------|
| id | UUID | PRIMARY KEY |
| project_id | UUID | NOT NULL |
| content | TEXT | NOT NULL |
| category | VARCHAR | |
| related_entity_ids | JSON | |
| version | INTEGER | NOT NULL DEFAULT 1 |
| created_at | TIMESTAMP | NOT NULL |
| updated_at | TIMESTAMP | NOT NULL |

### event
| Column | Type | Constraints |
|--------|------|-------------|
| id | UUID | PRIMARY KEY |
| project_id | UUID | NOT NULL |
| name | VARCHAR | NOT NULL |
| description | TEXT | NOT NULL |
| event_type | VARCHAR | |
| timestamp | VARCHAR | |
| event_time | VARCHAR | |
| duration | VARCHAR | |
| involved_entity_ids | JSON | |
| state_changes | JSON | |
| created_at | TIMESTAMP | NOT NULL |
| updated_at | TIMESTAMP | NOT NULL |

### state_change
| Column | Type | Constraints |
|--------|------|-------------|
| id | UUID | PRIMARY KEY |
| project_id | UUID | NOT NULL |
| event_id | UUID | |
| change_type | VARCHAR | NOT NULL |
| target_entity_id | UUID | NOT NULL |
| state_key | VARCHAR | NOT NULL |
| old_value | JSON | |
| new_value | JSON | NOT NULL |
| committed_at | TIMESTAMP | NOT NULL |
| committed_by | VARCHAR | |

### current_state
| Column | Type | Constraints |
|--------|------|-------------|
| id | UUID | PRIMARY KEY |
| project_id | UUID | NOT NULL |
| entity_id | UUID | NOT NULL |
| state_key | VARCHAR | NOT NULL |
| state_value | JSON | NOT NULL |
| effective_from | TIMESTAMP | NOT NULL |
| effective_to | TIMESTAMP | |
| version | INTEGER | NOT NULL DEFAULT 1 |
| created_at | TIMESTAMP | NOT NULL |
| updated_at | TIMESTAMP | NOT NULL |

### narrative_node
| Column | Type | Constraints |
|--------|------|-------------|
| id | UUID | PRIMARY KEY |
| project_id | UUID | NOT NULL |
| node_type | VARCHAR | NOT NULL |
| parent_id | UUID | |
| title | VARCHAR | NOT NULL |
| description | TEXT | |
| attributes | JSON | |
| sort_order | INTEGER | DEFAULT 0 |
| status | VARCHAR | NOT NULL DEFAULT 'Draft' |
| version | INTEGER | NOT NULL DEFAULT 1 |
| created_at | TIMESTAMP | NOT NULL |
| updated_at | TIMESTAMP | NOT NULL |

### knowledge_state
| Column | Type | Constraints |
|--------|------|-------------|
| id | UUID | PRIMARY KEY |
| project_id | UUID | NOT NULL |
| character_id | UUID | NOT NULL |
| entity_id | UUID | NOT NULL |
| knowledge_level | VARCHAR | NOT NULL |
| source | VARCHAR | |
| confidence | DOUBLE | DEFAULT 1.0 |
| created_at | TIMESTAMP | NOT NULL |
| updated_at | TIMESTAMP | NOT NULL |

### proposed_change
| Column | Type | Constraints |
|--------|------|-------------|
| id | UUID | PRIMARY KEY |
| project_id | UUID | NOT NULL |
| task_id | UUID | NOT NULL |
| change_type | VARCHAR | NOT NULL |
| target_entity_id | UUID | NOT NULL |
| description | TEXT | NOT NULL |
| payload | JSON | NOT NULL |
| status | VARCHAR | NOT NULL DEFAULT 'Draft' |
| created_at | TIMESTAMP | NOT NULL |
| resolved_at | TIMESTAMP | |

### validation_run
| Column | Type | Constraints |
|--------|------|-------------|
| id | UUID | PRIMARY KEY |
| project_id | UUID | NOT NULL |
| task_id | UUID | NOT NULL |
| changes_validated | INTEGER | NOT NULL |
| changes_approved | INTEGER | NOT NULL |
| changes_rejected | INTEGER | NOT NULL |
| status | VARCHAR | NOT NULL |
| started_at | TIMESTAMP | NOT NULL |
| completed_at | TIMESTAMP | |

### validation_issue
| Column | Type | Constraints |
|--------|------|-------------|
| id | UUID | PRIMARY KEY |
| validation_run_id | UUID | NOT NULL |
| proposed_change_id | UUID | NOT NULL |
| issue_type | VARCHAR | NOT NULL |
| severity | VARCHAR | NOT NULL |
| message | TEXT | NOT NULL |
| suggestion | TEXT | |
| created_at | TIMESTAMP | NOT NULL |

### job
| Column | Type | Constraints |
|--------|------|-------------|
| id | UUID | PRIMARY KEY |
| project_id | UUID | NOT NULL |
| job_type | VARCHAR | NOT NULL |
| name | VARCHAR | NOT NULL |
| description | TEXT | |
| status | VARCHAR | NOT NULL DEFAULT 'Pending' |
| priority | INTEGER | DEFAULT 0 |
| input | JSON | NOT NULL |
| output | JSON | |
| error | TEXT | |
| progress | DOUBLE | DEFAULT 0.0 |
| created_at | TIMESTAMP | NOT NULL |
| started_at | TIMESTAMP | |
| completed_at | TIMESTAMP | |

### generation_run
| Column | Type | Constraints |
|--------|------|-------------|
| id | UUID | PRIMARY KEY |
| project_id | UUID | NOT NULL |
| task_id | UUID | NOT NULL |
| context_snapshot_id | UUID | |
| llm_model | VARCHAR | NOT NULL |
| provider | VARCHAR | |
| prompt_sent | TEXT | NOT NULL |
| response_received | TEXT | NOT NULL |
| token_usage | JSON | |
| latency_ms | BIGINT | |
| skill_version | INTEGER | |
| prompt_version | INTEGER | |
| schema_version | INTEGER | |
| context_policy_version | INTEGER | |
| retry_count | INTEGER | DEFAULT 0 |
| max_retries | INTEGER | DEFAULT 3 |
| error | TEXT | |
| output_artifact_id | UUID | |
| created_at | TIMESTAMP | NOT NULL |

### context_snapshot
| Column | Type | Constraints |
|--------|------|-------------|
| id | UUID | PRIMARY KEY |
| project_id | UUID | NOT NULL |
| scene_id | UUID | NOT NULL |
| token_budget | INTEGER | NOT NULL |
| l0_essential | JSON | |
| l1_scene_relevant | JSON | |
| l2_recent_history | JSON | |
| l3_narrative_context | JSON | |
| l4_character_knowledge | JSON | |
| l5_world_background | JSON | |
| l6_optional_supplement | JSON | |
| actual_tokens | INTEGER | |
| content_hash | VARCHAR | |
| created_at | TIMESTAMP | NOT NULL |

### artifact
| Column | Type | Constraints |
|--------|------|-------------|
| id | UUID | PRIMARY KEY |
| project_id | UUID | NOT NULL |
| artifact_type | VARCHAR | NOT NULL |
| content_hash | VARCHAR | NOT NULL |
| storage_path | VARCHAR | NOT NULL |
| mime_type | VARCHAR | NOT NULL |
| size_bytes | BIGINT | NOT NULL |
| metadata | JSON | |
| created_at | TIMESTAMP | NOT NULL |

### domain_event
| Column | Type | Constraints |
|--------|------|-------------|
| id | UUID | PRIMARY KEY |
| event_type | VARCHAR | NOT NULL |
| project_id | UUID | NOT NULL |
| entity_id | UUID | |
| data | JSON | NOT NULL |
| metadata | JSON | NOT NULL |
| created_at | TIMESTAMP | NOT NULL |

### audit_log
| Column | Type | Constraints |
|--------|------|-------------|
| id | UUID | PRIMARY KEY |
| event_id | UUID | NOT NULL |
| action | VARCHAR | NOT NULL |
| entity_type | VARCHAR | NOT NULL |
| entity_id | UUID | NOT NULL |
| old_value | JSON | |
| new_value | JSON | |
| user_id | VARCHAR | |
| created_at | TIMESTAMP | NOT NULL |
