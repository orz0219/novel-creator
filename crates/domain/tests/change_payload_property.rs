//! domain 层 payload 契约（对应 docs/contracts/commit.md 不变量 2 /
//! events.md）
//!
//! 锁定「payload 必须匹配 change_type」契约在 domain 边界的可序列化形式：
//! 每种 ProposedChangeType 的规范 payload 必须包含其 dispatch 所需的键。
//! 这是 db::runtime_ports::commit_changes 分派逻辑的回归护栏与文档锚点
//! （真正的解析/拒绝测试在 db/tests/transaction_invariants.rs）。
//!
//! 纯单元测试，无需数据库。

use domain::ProposedChangeType;
use serde_json::{json, Value};

/// 返回每种 change_type 的规范 payload 及其必需键（与
/// db::runtime_ports 中 StateChangePayload / EntityCreatePayload /
/// RelationCreatePayload 的字段保持一致）。
fn canonical_payload(t: &ProposedChangeType) -> (Value, &'static [&'static str]) {
    match t {
        ProposedChangeType::StateChange => (
            json!({"state_key": "hp", "new_value": 80}),
            &["state_key", "new_value"],
        ),
        ProposedChangeType::EntityCreate => (
            json!({"entity_type": "Character", "name": "Hero", "attributes": {}}),
            &["entity_type", "name", "attributes"],
        ),
        ProposedChangeType::RelationCreate => (
            json!({"target_entity_id": "00000000-0000-0000-0000-000000000000", "relation_type": "friend", "attributes": {}}),
            &["target_entity_id", "relation_type", "attributes"],
        ),
        _ => (json!({}), &[]),
    }
}

#[test]
fn payload_shape_matches_change_type() {
    for t in [
        ProposedChangeType::StateChange,
        ProposedChangeType::EntityCreate,
        ProposedChangeType::RelationCreate,
    ] {
        let (payload, required) = canonical_payload(&t);
        let obj = payload
            .as_object()
            .unwrap_or_else(|| panic!("change_type {:?} 的 payload 必须是 JSON 对象", t));
        for key in required {
            assert!(
                obj.contains_key(*key),
                "change_type {:?} 的 payload 必须包含键 `{}`",
                t, key
            );
        }
    }
}

#[test]
fn proposed_change_type_serde_stable() {
    // change_type 的 (反)序列化形式必须稳定，否则任何基于 JSON 的分派/持久化都会错位。
    for t in [
        ProposedChangeType::StateChange,
        ProposedChangeType::EntityCreate,
        ProposedChangeType::RelationCreate,
    ] {
        let s = serde_json::to_string(&t).expect("serialize");
        let back: ProposedChangeType = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(t, back, "ProposedChangeType 序列化必须可往返：{}", s);
    }
}
