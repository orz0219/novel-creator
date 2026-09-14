//! Narrative - 叙事结构模型
//!
//! 统一的 NarrativeNode 树形结构，支持 Volume -> Arc -> Sequence -> Chapter -> Scene -> Beat 层级。
//! 使用单一表 + node_type 枚举实现，便于未来扩展新的节点类型。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// 叙事节点类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum NarrativeNodeType {
    /// 卷
    Volume,
    /// 故事弧线
    Arc,
    /// 序列
    Sequence,
    /// 章节
    Chapter,
    /// 场景
    Scene,
    /// 节拍（场景内的最小生成单位）
    Beat,
    /// 故事线（可选，用于并行叙事）
    Storyline,
    /// 子弧线（可选）
    SubArc,
    /// 特殊节点（可选）
    Special,
    /// 自定义类型：只有显式写 `custom:名字` 才走这里。
    ///
    /// 为什么留这条通道：卷/弧/章/场/节拍是内置层级，但作者确实会有自己的切片
    /// （实测里出现过「支线碎片」「过场」）。既要能自定义，又要有护栏——
    /// 所以**必须显式带 `custom:` 前缀**：`chaptr` 这种拼错会直接报错，
    /// 而不是悄悄多出一种节点类型。
    Custom(String),
}

/// 自定义节点类型的前缀（中英文都可写）。
pub const NODE_TYPE_CUSTOM_PREFIXES: &[&str] = &["custom:", "自定义:"];

impl NarrativeNodeType {
    /// 内置取值（英文规范名 + 中文别名）。错误文案、工具 schema、前端词表共用这一份。
    pub const CANONICAL: &'static [(&'static str, &'static str)] = &[
        ("Volume", "卷"),
        ("Arc", "弧"),
        ("Sequence", "序列"),
        ("Chapter", "章"),
        ("Scene", "场"),
        ("Beat", "节拍"),
        ("Storyline", "故事线"),
        ("SubArc", "子弧"),
        ("Special", "特殊"),
    ];

    /// 存库字符串（自定义类型带 `custom:` 前缀）。
    pub fn as_db_str(&self) -> String {
        match self {
            NarrativeNodeType::Volume => "Volume".into(),
            NarrativeNodeType::Arc => "Arc".into(),
            NarrativeNodeType::Sequence => "Sequence".into(),
            NarrativeNodeType::Chapter => "Chapter".into(),
            NarrativeNodeType::Scene => "Scene".into(),
            NarrativeNodeType::Beat => "Beat".into(),
            NarrativeNodeType::Storyline => "Storyline".into(),
            NarrativeNodeType::SubArc => "SubArc".into(),
            NarrativeNodeType::Special => "Special".into(),
            NarrativeNodeType::Custom(name) => format!("custom:{}", name),
        }
    }

    /// 严格解析：内置词表（英文规范名 / 常见简写 / 中文别名，忽略大小写、下划线与空白）
    /// 或显式自定义（`custom:名字`）。**其余一律报错**。
    ///
    /// 与旧行为的分界：以前未知取值会被静默当成 `Scene`
    /// （`db::ser::parse_narrative_node_type`），于是拼错的类型写进库、读回来变成
    /// 另一种类型，两头都不报错。现在拼错就是拼错，错误里直接列出合法取值。
    pub fn parse_strict(input: &str) -> anyhow::Result<Self> {
        let raw = input.trim();
        if raw.is_empty() {
            anyhow::bail!("node_type 为空；{}", Self::legal_values_hint());
        }

        for prefix in NODE_TYPE_CUSTOM_PREFIXES {
            if let Some(rest) = raw.strip_prefix(prefix) {
                let name = rest.trim();
                if name.is_empty() {
                    anyhow::bail!("node_type 的 {} 前缀后面要有名字，例如 custom:过场", prefix);
                }
                return Ok(NarrativeNodeType::Custom(name.to_string()));
            }
        }

        let key = raw.to_ascii_lowercase().replace(['_', '-', ' '], "");
        let parsed = match key.as_str() {
            "volume" | "vol" | "卷" | "分卷" | "卷册" => NarrativeNodeType::Volume,
            "arc" | "弧" | "弧线" | "故事弧" | "大弧" => NarrativeNodeType::Arc,
            "sequence" | "序列" => NarrativeNodeType::Sequence,
            "chapter" | "chap" | "ch" | "章" | "章节" => NarrativeNodeType::Chapter,
            "scene" | "场" | "场景" => NarrativeNodeType::Scene,
            "beat" | "节拍" | "拍" | "节" => NarrativeNodeType::Beat,
            "storyline" | "line" | "故事线" | "剧情线" | "线" => NarrativeNodeType::Storyline,
            "subarc" | "子弧" | "子弧线" | "副弧" => NarrativeNodeType::SubArc,
            "special" | "特殊" | "特殊节点" => NarrativeNodeType::Special,
            _ => anyhow::bail!("node_type 不认识：{}。{}", raw, Self::legal_values_hint()),
        };
        Ok(parsed)
    }

    /// 合法取值提示（错误文案与工具 schema 共用一份，避免两处词表各说各话）。
    pub fn legal_values_hint() -> String {
        let list = Self::CANONICAL
            .iter()
            .map(|(en, zh)| format!("{}（{}）", en, zh))
            .collect::<Vec<_>>()
            .join(" / ");
        format!(
            "合法取值：{}；自有词表请显式写成 custom:名字（例如 custom:过场）",
            list
        )
    }
}

/// 一条「节点服务哪条故事线的哪个阶段」的引用。
///
/// 形状与 `storyline.arc_stages[].stage` 对齐：阶段用名字（不是序号），
/// 因为阶段表本身就是按名字读写的。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NarrativeStageRef {
    pub storyline_id: Uuid,
    /// 阶段名；None 表示只挂线、不指定阶段
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arc_stage: Option<String>,
}

/// 叙事节点的**结构与挂载**补丁（细纲专用）。
///
/// 字段语义统一为「None = 不改」；需要「置空」的用显式的 clear 标志，
/// 因为 `Option<Option<T>>` 在 JSON 往返里分不清「不改」和「改成 null」。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct NarrativeNodeOutlinePatch {
    /// 改节点类型（严格词表校验，见 [`NarrativeNodeType::parse_strict`]）
    pub node_type: Option<String>,
    /// 换父节点
    pub parent_id: Option<Uuid>,
    /// 移到顶层（parent_id = NULL）；与 `parent_id` 同时给会报错
    pub move_to_root: bool,
    /// 同父下的目标序号（1 起）。给了就做**兄弟重排**：同父其他节点自动顺移。
    pub sort_order: Option<i32>,
    /// 主挂载：故事线
    pub storyline_id: Option<Uuid>,
    /// 解绑故事线
    pub clear_storyline: bool,
    /// 主挂载：阶段名
    pub arc_stage: Option<String>,
    /// 附加挂载：整体替换（空数组 = 清空）
    pub stage_refs: Option<Vec<NarrativeStageRef>>,
    /// 场景挂载：在场角色（整体替换）
    pub participant_entity_ids: Option<Vec<Uuid>>,
    /// 场景挂载：地点
    pub location_id: Option<Uuid>,
    /// 解绑地点
    pub clear_location: bool,
    /// 场景挂载：道具（整体替换）
    pub item_ids: Option<Vec<Uuid>>,
    /// 元数据：预计章数
    pub estimated_chapters: Option<i32>,
    /// 元数据：预计字数
    pub estimated_words: Option<i32>,
    /// 元数据：故事内时间跨度
    pub story_time: Option<String>,
}

impl NarrativeNodeOutlinePatch {
    /// 是否什么都没改（工具层据此提示「这次 revise 没有实际改动」）。
    pub fn is_empty(&self) -> bool {
        self.node_type.is_none()
            && self.parent_id.is_none()
            && !self.move_to_root
            && self.sort_order.is_none()
            && self.storyline_id.is_none()
            && !self.clear_storyline
            && self.arc_stage.is_none()
            && self.stage_refs.is_none()
            && self.participant_entity_ids.is_none()
            && self.location_id.is_none()
            && !self.clear_location
            && self.item_ids.is_none()
            && self.estimated_chapters.is_none()
            && self.estimated_words.is_none()
            && self.story_time.is_none()
    }
}

/// 叙事节点 - 统一的树形结构
///
/// 所有叙事层级（卷/弧/序列/章/场景/节拍）都用同一个结构表示。
/// 通过 node_type 区分层级，通过 parent_id 建立树形关系。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeNode {
    pub id: Uuid,
    pub project_id: Uuid,
    pub world_id: Uuid,
    pub node_type: NarrativeNodeType,
    pub parent_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    /// 场景/章节正文（编辑器草稿内容）。Volume/Arc 等也可承载概述。
    pub content: Option<String>,
    /// 节点特有的属性（JSON），不同类型有不同结构
    pub attributes: serde_json::Value,
    pub sort_order: i32,
    pub status: NarrativeNodeStatus,
    /// 主挂载：这条节点服务的故事线
    pub storyline_id: Option<Uuid>,
    /// 主挂载：推进到这条故事线的哪个阶段（与 `storyline.arc_stages[].stage` 同名）
    pub arc_stage: Option<String>,
    /// 附加挂载：一个节点可能同时服务多条线 / 多个阶段
    pub stage_refs: Vec<NarrativeStageRef>,
    /// 场景级挂载：在场的角色（entity id）
    pub participant_entity_ids: Vec<Uuid>,
    /// 场景级挂载：发生地点（entity id）
    pub location_id: Option<Uuid>,
    /// 场景级挂载：用到的道具（entity id）
    pub item_ids: Vec<Uuid>,
    /// 元数据：预计章数
    pub estimated_chapters: Option<i32>,
    /// 元数据：预计字数
    pub estimated_words: Option<i32>,
    /// 元数据：故事内时间跨度（自由文本，例如「第三天黄昏」）
    pub story_time: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 叙事节点状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NarrativeNodeStatus {
    /// 草稿
    Draft,
    /// 已规划
    Planned,
    /// 进行中
    InProgress,
    /// 已完成
    Completed,
    /// 已归档
    Archived,
}

impl NarrativeNodeStatus {
    /// 内置取值（英文规范名 + 中文别名）。
    pub const CANONICAL: &'static [(&'static str, &'static str)] = &[
        ("Draft", "草稿"),
        ("Planned", "已规划"),
        ("InProgress", "进行中"),
        ("Completed", "已完成"),
        ("Archived", "已归档"),
    ];

    pub fn as_db_str(&self) -> String {
        match self {
            NarrativeNodeStatus::Draft => "Draft".into(),
            NarrativeNodeStatus::Planned => "Planned".into(),
            NarrativeNodeStatus::InProgress => "InProgress".into(),
            NarrativeNodeStatus::Completed => "Completed".into(),
            NarrativeNodeStatus::Archived => "Archived".into(),
        }
    }

    /// 严格解析（忽略大小写、下划线与空白）。未知取值报错，不静默落回 `Draft`。
    ///
    /// 注意：软删除用的 `Deleted` **不是**这个枚举的取值——删除行由 SQL 的
    /// `status != 'Deleted'` 过滤，不该出现在读取路径上；真读到了就说明查询漏了过滤，
    /// 那时候报错比静默当成 `Draft` 正确。
    pub fn parse_strict(input: &str) -> anyhow::Result<Self> {
        let raw = input.trim();
        let key = raw.to_ascii_lowercase().replace(['_', '-', ' '], "");
        let parsed = match key.as_str() {
            "draft" | "草稿" | "草案" => NarrativeNodeStatus::Draft,
            "planned" | "已规划" | "规划" => NarrativeNodeStatus::Planned,
            "inprogress" | "进行中" | "在写" => NarrativeNodeStatus::InProgress,
            "completed" | "已完成" | "完成" => NarrativeNodeStatus::Completed,
            "archived" | "已归档" | "归档" => NarrativeNodeStatus::Archived,
            _ => anyhow::bail!(
                "status 不认识：{}。合法取值：{}",
                raw,
                Self::CANONICAL
                    .iter()
                    .map(|(en, zh)| format!("{}（{}）", en, zh))
                    .collect::<Vec<_>>()
                    .join(" / ")
            ),
        };
        Ok(parsed)
    }
}

/// 卷的扩展属性
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct VolumeAttributes {
    pub mission: Option<String>,
    pub theme: Option<String>,
    pub conflict: Option<String>,
    pub goal: Option<String>,
    pub start_state: Option<String>,
    pub end_state: Option<String>,
    pub important_character_ids: Vec<Uuid>,
    pub important_location_ids: Vec<Uuid>,
    pub major_events: Vec<String>,
    pub secrets: Vec<String>,
    pub foreshadowing: Vec<String>,
    pub resolution: Option<String>,
    /// Story Contract ID for this volume
    pub story_contract_id: Option<Uuid>,
}

/// 弧线的扩展属性
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ArcAttributes {
    /// 弧线目标
    pub goal: Option<String>,
    /// 核心冲突
    pub conflict: Option<String>,
    /// 参与者
    pub participants: Vec<String>,
    /// 开始条件
    pub start_condition: Option<String>,
    /// 结束条件
    pub end_condition: Option<String>,
    /// 关键事件
    pub key_events: Vec<String>,
    /// 转折点
    pub twists: Vec<String>,
    /// Story Contract ID for this arc
    pub story_contract_id: Option<Uuid>,
}

/// 场景的扩展属性
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SceneAttributes {
    pub objective: Option<String>,
    pub conflict: Option<String>,
    pub pov_character_id: Option<Uuid>,
    pub location_id: Option<Uuid>,
    pub time: Option<String>,
    /// 情绪目标（这一幕要传达什么情绪）
    pub emotional_goal: Option<String>,
    /// 信息目标（这一幕要让读者/角色知道什么）
    pub information_goal: Option<String>,
    pub required_events: Vec<String>,
    /// 禁止发生的事件
    pub forbidden_events: Vec<String>,
    /// 预期的世界状态变化
    pub expected_changes: Vec<String>,
    pub required_facts: Vec<String>,
    pub characters_present: Vec<Uuid>,
}

/// 节拍的扩展属性
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct BeatAttributes {
    pub action: String,
    pub emotion: Option<String>,
    pub dialogue_needed: bool,
    pub word_count_target: Option<i32>,
}

/// 场景 - 叙事节点的具体化视图
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub id: Uuid,
    pub narrative_node_id: Uuid,
    pub objective: Option<String>,
    pub conflict: Option<String>,
    pub pov_character_id: Option<Uuid>,
    pub location_id: Option<Uuid>,
    pub time: Option<String>,
    pub scene_start_time: Option<String>,
    pub scene_end_time: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 场景中涉及的实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneEntity {
    pub id: Uuid,
    pub scene_id: Uuid,
    pub entity_id: Uuid,
    pub role: Option<String>,
    pub notes: Option<String>,
}

/// 场景需求 - 该场景必须满足的条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneRequirement {
    pub id: Uuid,
    pub scene_id: Uuid,
    pub requirement_type: String,
    pub content: String,
    pub priority: RequirementPriority,
}

/// 需求优先级
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RequirementPriority {
    Must,
    Should,
    Could,
}

/// 角色弧线 - 追踪角色在叙事中的成长/变化
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterArc {
    pub id: Uuid,
    pub project_id: Uuid,
    pub character_id: Uuid,
    pub volume_id: Option<Uuid>,
    pub arc_type: String,
    pub start_state: Option<String>,
    pub mid_state: Option<String>,
    pub end_state: Option<String>,
    pub key_moments: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// NarrativeState - 叙事状态（已从人物模块移出，归叙事引擎）
///
/// 区分 World State（世界发生了什么）和 Narrative State（叙事已揭示什么）。
/// 例如：World State = 王家已灭亡，Narrative State = 读者不知道王家已灭亡。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeState {
    pub id: Uuid,
    pub project_id: Uuid,
    /// 状态维度（World/Narrative/Character/Reader）
    pub state_dimension: StateDimension,
    /// 状态键
    pub state_key: String,
    /// 状态值
    pub state_value: Value,
    /// 关联的场景 ID
    pub scene_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 状态维度
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StateDimension {
    /// 世界状态（客观事实）
    World,
    /// 叙事状态（已揭示给读者的）
    Narrative,
    /// 角色状态（角色当前状态）
    Character,
    /// 读者状态（读者当前知道的）
    Reader,
}

impl StateDimension {
    pub fn as_str(&self) -> &'static str {
        match self {
            StateDimension::World => "World",
            StateDimension::Narrative => "Narrative",
            StateDimension::Character => "Character",
            StateDimension::Reader => "Reader",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "World" => StateDimension::World,
            "Narrative" => StateDimension::Narrative,
            "Character" => StateDimension::Character,
            "Reader" => StateDimension::Reader,
            _ => StateDimension::World,
        }
    }
}

/// 新建叙事节点的入参。
///
/// 细纲字段多（结构与挂载十余项），用结构体而不是十几个位置参数——
/// 位置参数一旦再加一列，所有调用点都要跟着改，而且容易传错位。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewNarrativeNode {
    pub project_id: Uuid,
    /// 节点类型（严格词表，见 [`NarrativeNodeType::parse_strict`]）
    pub node_type: String,
    pub parent_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    /// 章节 / 节点正文（可选）
    pub content: Option<String>,
    pub attributes: serde_json::Value,
    /// 同父下的目标序号（1 起）；None = 追加到同父末尾
    pub sort_order: Option<i32>,
    /// 初始状态（严格词表）；None = Draft
    pub status: Option<String>,
    pub storyline_id: Option<Uuid>,
    pub arc_stage: Option<String>,
    pub stage_refs: Vec<NarrativeStageRef>,
    pub participant_entity_ids: Vec<Uuid>,
    pub location_id: Option<Uuid>,
    pub item_ids: Vec<Uuid>,
    pub estimated_chapters: Option<i32>,
    pub estimated_words: Option<i32>,
    pub story_time: Option<String>,
}

/// 叙事节点列表过滤条件（细纲树用）。
///
/// 为什么需要它：节点树到 300 章时拉全量自己拼树已经拉不动了——
/// 至少得能按父节点下钻、按类型筛、按故事线筛。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct NarrativeNodeFilter {
    /// 只看该父节点下的直接子节点
    pub parent_id: Option<Uuid>,
    /// 只看顶层节点（parent_id IS NULL）；与 parent_id 同时给会报错
    pub roots_only: bool,
    /// 只看某种节点类型（严格词表）
    pub node_type: Option<String>,
    /// 只看服务某条故事线的节点
    pub storyline_id: Option<Uuid>,
}
