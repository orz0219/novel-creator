//! E2E Tests - 完整流水线测试
//!
//! 测试设计稿3的所有新增功能：Timeline + Storyline + Visibility + Approval + Foreshadowing + Branch

use db::connection::Database;
use db::migration;
use domain::*;
use uuid::Uuid;

fn setup_db() -> Database {
    let db = Database::open_in_memory().unwrap();
    migration::run_migrations(&db, concat!(env!("CARGO_MANIFEST_DIR"), "/../db/migrations")).unwrap();
    db
}

fn create_project(db: &Database) -> Uuid {
    let project_id = Uuid::new_v4();
    {
        let conn = db.conn();
        conn.execute(
            "INSERT INTO project (id, name, description, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
            [project_id.to_string(), "Test Novel".to_string(), "A test novel".to_string(), "Active".to_string(), chrono::Utc::now().to_string(), chrono::Utc::now().to_string()],
        ).unwrap();
    }
    project_id
}

#[test]
fn test_e2e_storyline_lifecycle() {
    let db = setup_db();
    let project_id = create_project(&db);
    let repo = db::repos::storyline_repo::StorylineRepo::new(&db);

    // 创建剧情线
    let storyline = repo.create(project_id, "地下遗迹真相", Some("贯穿多卷的核心悬念"), StorylineImportance::Main).unwrap();
    assert_eq!(storyline.status, StorylineStatus::Active);

    // 关联场景
    let scene_id = Uuid::new_v4();
    let link = repo.link_scene(storyline.id, scene_id, Some("伏笔引入")).unwrap();
    assert_eq!(link.storyline_id, storyline.id);
}

#[test]
fn test_e2e_visibility_control() {
    let db = setup_db();
    let project_id = create_project(&db);
    let repo = db::repos::visibility_repo::VisibilityRepo::new(&db);

    // 创建事实
    let fact_id = Uuid::new_v4();
    let conn = db.conn();
    conn.execute(
        "INSERT INTO fact (id, project_id, content, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
        [fact_id.to_string(), project_id.to_string(), "幕后黑手是A".to_string(), chrono::Utc::now().to_string(), chrono::Utc::now().to_string()],
    ).unwrap();

    // 设置可见性：作者可见，场景作者隐藏
    repo.create(project_id, fact_id, VisibilitySubjectType::Author, None, VisibilityLevel::Visible).unwrap();
    repo.create(project_id, fact_id, VisibilitySubjectType::SceneWriter, None, VisibilityLevel::Hidden).unwrap();

    // 验证可见性
    let author_level = repo.check_visibility(fact_id, VisibilitySubjectType::Author, None).unwrap();
    assert_eq!(author_level, VisibilityLevel::Visible);

    let writer_level = repo.check_visibility(fact_id, VisibilitySubjectType::SceneWriter, None).unwrap();
    assert_eq!(writer_level, VisibilityLevel::Hidden);
}

#[test]
fn test_e2e_approval_gate() {
    let db = setup_db();
    let project_id = create_project(&db);
    let repo = db::repos::approval_repo::ApprovalRepo::new(&db);

    // AI 提案创建新角色
    let target_id = Uuid::new_v4();
    let record = repo.create(project_id, ApprovalTargetType::Entity, target_id, "ai", serde_json::json!({
        "name": "地下赌场老板",
        "type": "Character",
        "description": "黑市的神秘老板"
    })).unwrap();
    assert_eq!(record.status, ApprovalStatus::Pending);

    // 检查待审批列表
    let pending = repo.list_pending(project_id).unwrap();
    assert_eq!(pending.len(), 1);

    // 用户批准
    repo.approve(record.id, "author1", Some("角色设定合理")).unwrap();

    // 验证已批准
    let pending_after = repo.list_pending(project_id).unwrap();
    assert_eq!(pending_after.len(), 0);
}

#[test]
fn test_e2e_foreshadowing_tracking() {
    let db = setup_db();
    let project_id = create_project(&db);
    let repo = db::repos::foreshadowing_repo::ForeshadowingRepo::new(&db);

    // 创建伏笔
    let f = repo.create(project_id, "奇怪石碑", Some("第5章出现的神秘石碑"), ForeshadowingImportance::Important, HintLevel::Hidden).unwrap();
    assert_eq!(f.status, ForeshadowingStatus::Planned);

    // 引入伏笔
    repo.update_status(f.id, ForeshadowingStatus::Introduced).unwrap();

    // 验证状态更新
    let list = repo.list_by_project(project_id).unwrap();
    assert_eq!(list[0].status, ForeshadowingStatus::Introduced);
}

#[test]
fn test_e2e_branch_system() {
    let db = setup_db();
    let project_id = create_project(&db);
    let repo = db::repos::branch_repo::BranchRepo::new(&db);

    // 创建主分支
    let main = repo.create_world_branch(project_id, "main", Some("主线世界"), true).unwrap();
    assert!(main.is_main);

    // 创建替代分支
    let alt = repo.create_world_branch(project_id, "chapter-20-rewrite", Some("第20章重写"), false).unwrap();
    assert!(!alt.is_main);

    // 列出所有分支
    let branches = repo.list_world_branches(project_id).unwrap();
    assert_eq!(branches.len(), 2);
}

#[test]
fn test_e2e_quality_scoring() {
    let db = setup_db();
    let project_id = create_project(&db);
    let repo = db::repos::quality_repo::QualityScoreRepo::new(&db);

    // 创建质量评分
    let scene_id = Uuid::new_v4();
    let issues = vec![
        QualityIssue { dimension: "style".into(), severity: "Warning".into(), description: "节奏偏慢".into(), suggestion: Some("加快节奏".into()) },
    ];
    let qs = repo.create(project_id, scene_id, Some(96), Some(91), Some(100), Some(100), Some(97), Some(87), issues).unwrap();
    
    // 验证综合评分
    assert_eq!(qs.overall_score, Some(95));
    assert_eq!(qs.issues.len(), 1);
}

#[test]
fn test_e2e_causal_chain() {
    let db = setup_db();
    let project_id = create_project(&db);
    let repo = db::repos::causal_repo::CausalRepo::new(&db);

    // 创建因果关系
    let cause_event = Uuid::new_v4();
    let effect_event = Uuid::new_v4();
    let c = repo.create(project_id, cause_event, effect_event, CausalRelationType::DirectCause, CausalStrength::Strong, Some("资金不足导致扩张")).unwrap();
    assert_eq!(c.relation_type, CausalRelationType::DirectCause);

    // 列出因果关系
    let list = repo.list_by_project(project_id).unwrap();
    assert_eq!(list.len(), 1);
}

#[test]
fn test_e2e_reader_knowledge() {
    let db = setup_db();
    let project_id = create_project(&db);
    let repo = db::repos::reader_repo::ReaderKnowledgeRepo::new(&db);

    // 创建读者知识
    let fact_id = Uuid::new_v4();
    let rk = repo.create(project_id, fact_id, ReaderKnowledgeLevel::Suspected, ReaderConfidence::Speculative).unwrap();
    assert_eq!(rk.knowledge_level, ReaderKnowledgeLevel::Suspected);

    // 列出读者知识
    let list = repo.list_by_project(project_id).unwrap();
    assert_eq!(list.len(), 1);
}

#[test]
fn test_e2e_contract_validation() {
    let db = setup_db();
    let repo = db::repos::contract_repo::ContractRepo::new(&db);

    // 创建场景契约
    let scene_id = Uuid::new_v4();
    let contract = repo.create(
        scene_id,
        vec!["进入黑市".into(), "遇到老板".into()],
        vec!["发现遗迹".into(), "击杀王家".into()],
        vec![],
        vec!["黑市存在".into()],
        vec!["黑市存在王家眼线".into()],
        vec!["王家正在调查自己".into()],
        vec!["获得通行资格".into()],
    ).unwrap();

    // 验证契约
    let validator = runtime::contract_validator::ContractValidator::new(&db);
    
    // 通过的草稿
    let draft_pass = "林凡进入黑市，遇到了老板，了解了黑市的情况。他获得了通行资格。";
    let result_pass = validator.validate(&contract, draft_pass).unwrap();
    assert!(result_pass.passed);

    // 违反禁止事件的草稿
    let draft_fail = "林凡进入黑市，突然发现了远古遗迹的入口。";
    let result_fail = validator.validate(&contract, draft_fail).unwrap();
    assert!(!result_fail.passed);
    assert!(!result_fail.forbidden_events_violated.is_empty());
}
