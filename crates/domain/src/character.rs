//! Character 子结构 - 通用小说人物设定模型
//!
//! 设计原则（来自与 ChatGPT 协作修订的 R2 方案）：
//! - 人物是「剧情发动机」，每个字段都要能直接或间接驱动情节；
//! - 跨题材通用：标量身份约束 + 剧情核心 + 题材扩展(Extension) 三层；
//! - 不绑定单一题材（玄幻/都市/科幻/悬疑/言情共用同一模块）；
//! - 不是真人档案，也不是心理测评，是程序可读、供自动小说引擎消费的结构化数据。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ===================== 通用枚举 =====================

/// 叙事年龄：生成用的年龄区间，不是真实年龄
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgeRange {
    Child,
    Teen,
    YoungAdult,
    Adult,
    MiddleAge,
    Elder,
    Unknown,
}

impl AgeRange {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgeRange::Child => "Child",
            AgeRange::Teen => "Teen",
            AgeRange::YoungAdult => "YoungAdult",
            AgeRange::Adult => "Adult",
            AgeRange::MiddleAge => "MiddleAge",
            AgeRange::Elder => "Elder",
            AgeRange::Unknown => "Unknown",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "Child" => AgeRange::Child,
            "Teen" => AgeRange::Teen,
            "YoungAdult" => AgeRange::YoungAdult,
            "Adult" => AgeRange::Adult,
            "MiddleAge" => AgeRange::MiddleAge,
            "Elder" => AgeRange::Elder,
            _ => AgeRange::Unknown,
        }
    }

    /// 严格解析：无法识别的值返回 `None`，**不静默兜底**。
    ///
    /// 同时接受中文写法：这是中文小说创作系统，作者与模型都会自然地写「青年」。
    /// 认识的写法明确映射到规范值，不认识的才报错——「认识但写法不同」与
    /// 「根本不知道你在说什么」必须区别对待，前者不该被拒绝。
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "Child" | "儿童" | "孩童" | "小孩" => Some(AgeRange::Child),
            "Teen" | "少年" | "青少年" | "青春期" => Some(AgeRange::Teen),
            "YoungAdult" | "Young Adult" | "青年" | "年轻人" => Some(AgeRange::YoungAdult),
            "Adult" | "成年" | "成年人" => Some(AgeRange::Adult),
            "MiddleAge" | "MiddleAged" | "Middle Age" | "middle-aged" | "中年" | "中年人" => {
                Some(AgeRange::MiddleAge)
            }
            "Elder" | "Elderly" | "老年" | "老人" | "老年期" => Some(AgeRange::Elder),
            "Unknown" | "不明" | "未知" => Some(AgeRange::Unknown),
            _ => None,
        }
    }

    /// 全部合法取值，供工具的 JSON Schema 与错误提示复用。
    pub const ALL: [&'static str; 7] = [
        "Child",
        "Teen",
        "YoungAdult",
        "Adult",
        "MiddleAge",
        "Elder",
        "Unknown",
    ];
}

/// 性别：定位为身份约束(identity_constraint)，不是性格
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Gender {
    Male,
    Female,
    NonBinary,
    Unknown,
    Other,
}

impl Gender {
    pub fn as_str(&self) -> &'static str {
        match self {
            Gender::Male => "Male",
            Gender::Female => "Female",
            Gender::NonBinary => "NonBinary",
            Gender::Unknown => "Unknown",
            Gender::Other => "Other",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "Male" => Gender::Male,
            "Female" => Gender::Female,
            "NonBinary" => Gender::NonBinary,
            "Unknown" => Gender::Unknown,
            _ => Gender::Other,
        }
    }

    /// 严格解析（语义同 `AgeRange::parse`）：非法值返回 `None` 而不是降级成 `Other`；
    /// 同时接受中文写法。
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "Male" | "男" | "男性" => Some(Gender::Male),
            "Female" | "女" | "女性" => Some(Gender::Female),
            "NonBinary" | "非二元" | "其他性别" => Some(Gender::NonBinary),
            "Unknown" | "不明" | "未知" => Some(Gender::Unknown),
            "Other" | "其他" => Some(Gender::Other),
            _ => None,
        }
    }

    pub const ALL: [&'static str; 5] = ["Male", "Female", "NonBinary", "Unknown", "Other"];
}

/// 角色在故事中的功能位（role_in_story）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum StoryRole {
    Protagonist,
    Antagonist,
    Mentor,
    Ally,
    Rival,
    Catalyst,
    Victim,
    Observer,
}

impl StoryRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            StoryRole::Protagonist => "Protagonist",
            StoryRole::Antagonist => "Antagonist",
            StoryRole::Mentor => "Mentor",
            StoryRole::Ally => "Ally",
            StoryRole::Rival => "Rival",
            StoryRole::Catalyst => "Catalyst",
            StoryRole::Victim => "Victim",
            StoryRole::Observer => "Observer",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "Protagonist" => StoryRole::Protagonist,
            "Antagonist" => StoryRole::Antagonist,
            "Mentor" => StoryRole::Mentor,
            "Ally" => StoryRole::Ally,
            "Rival" => StoryRole::Rival,
            "Catalyst" => StoryRole::Catalyst,
            "Victim" => StoryRole::Victim,
            _ => StoryRole::Observer,
        }
    }

    /// 严格解析（语义同 `AgeRange::parse`）：非法值返回 `None` 而不是降级成 `Observer`；
    /// 同时接受中文写法。
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "Protagonist" | "主角" => Some(StoryRole::Protagonist),
            "Antagonist" | "反派" | "反面角色" => Some(StoryRole::Antagonist),
            "Mentor" | "导师" | "引路人" => Some(StoryRole::Mentor),
            "Ally" | "盟友" | "伙伴" | "配角" | "辅助角色" => Some(StoryRole::Ally),
            "Rival" | "对手" | "竞争者" => Some(StoryRole::Rival),
            "Catalyst" | "催化剂" | "推动者" => Some(StoryRole::Catalyst),
            "Victim" | "受害者" => Some(StoryRole::Victim),
            "Observer" | "旁观者" | "观察者" => Some(StoryRole::Observer),
            _ => None,
        }
    }

    pub const ALL: [&'static str; 8] = [
        "Protagonist",
        "Antagonist",
        "Mentor",
        "Ally",
        "Rival",
        "Catalyst",
        "Victim",
        "Observer",
    ];
}

/// 冲突类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConflictType {
    Internal,
    External,
    Relationship,
    Ideology,
}

impl ConflictType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConflictType::Internal => "Internal",
            ConflictType::External => "External",
            ConflictType::Relationship => "Relationship",
            ConflictType::Ideology => "Ideology",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "Internal" => ConflictType::Internal,
            "External" => ConflictType::External,
            "Relationship" => ConflictType::Relationship,
            "Ideology" => ConflictType::Ideology,
            _ => ConflictType::Internal,
        }
    }

    /// 严格解析，同时接受中文（语义同 `AgeRange::parse`）：
    /// 不认识的写法返回 `None`，而不是一律降级成 `Internal`。
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "Internal" | "内在" | "内心" | "内部" => Some(ConflictType::Internal),
            "External" | "外在" | "外部" | "环境" => Some(ConflictType::External),
            "Relationship" | "关系" | "人际" => Some(ConflictType::Relationship),
            "Ideology" | "理念" | "信仰" | "观念" => Some(ConflictType::Ideology),
            _ => None,
        }
    }

    pub const ALL: [&'static str; 4] = ["Internal", "External", "Relationship", "Ideology"];
}

// ===================== 组合子结构 =====================

/// 社会位置（social_status 的抽象：不写死"贵族/平民"）
///
/// `serde(default)` 是必需的：`social_access` 不是 Option，若没有它，
/// 调用方只提供 `rank` 时反序列化会因 "missing field `social_access`" 直接失败，
/// 表现为前端保存社会地位时整个请求 500。缺失即"没有"，用默认值即可。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SocialPosition {
    pub rank: Option<String>,
    pub authority_level: Option<i32>,
    pub social_access: Vec<String>,
}

/// 剧情必要性（R2 新增）：告诉引擎谁该重点写、谁可以死、谁可替换
///
/// 同 `SocialPosition`：`importance` / `irreplaceability` 不是 Option，
/// 必须允许部分字段缺失。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct NarrativeNecessity {
    pub importance: i32,
    pub irreplaceability: i32,
    pub absence_effect: Option<String>,
    pub replacement_cost: Option<String>,
}

fn default_urgency() -> i32 {
    3
}

/// 驱动力（合并原 CharacterGoal 的多级目标，并补上恐惧/弱点/欲望/矛盾）
///
/// `serde(default)` 必须保留：`urgency` 不是 Option，若模型只给动机/目标
/// 而没写 urgency，反序列化会因 "missing field `urgency`" 直接失败。
/// 缺失时默认 3（中等紧迫度），不把模型逼进无意义的字段重试。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CharacterDrive {
    pub primary_goal: Option<String>,
    pub motivation: Option<String>,
    #[serde(default = "default_urgency")]
    pub urgency: i32,
    pub long_term: Option<String>,
    pub current: Option<String>,
    pub immediate: Option<String>,
    pub hidden_goal: Option<String>,
    pub fear: Option<String>,
    pub weakness: Option<String>,
    pub desire: Option<String>,
    pub contradiction: Option<String>,
}

/// 冲突（人物真正参与剧情的接口）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterConflict {
    pub id: Uuid,
    pub entity_id: Uuid,
    pub conflict_type: ConflictType,
    pub description: String,
    pub target_entity_id: Option<Uuid>,
    pub resolution_status: Option<String>,
    /// 该冲突从哪个阶段（`EntityArcStage.stage`）开始成立。
    ///
    /// 为什么需要它：周浩「怕被排除在外」这条冲突在主角开口之前根本不存在；
    /// 不分阶段的话，它会从第一章起就成立。前端可按阶段过滤，引擎可只取当下生效的冲突。
    pub phase: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 关系（支撑联盟/背叛/冲突/羁绊）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterRelationship {
    pub id: Uuid,
    pub entity_id: Uuid,
    pub target_entity_id: Uuid,
    pub relationship_type: String,
    pub attitude: String,
    pub trust_level: i32,
    pub secret_knowledge: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 秘密（驱动悬念/反转/信息差）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterSecret {
    pub id: Uuid,
    pub entity_id: Uuid,
    pub content: String,
    pub importance: i32,
    pub reveal_condition: Option<String>,
    pub related_entities: Vec<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 能力边界（限制比能力更重要）
///
/// `serde(default)` 让 skills / limitations 可以省略其一：
/// 模型只写 skills 时不再因 missing field 直接失败。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CharacterCapability {
    pub skills: Vec<String>,
    pub limitations: Vec<String>,
}

/// 弧光潜力（人物成长线）—— 内在曲线：为什么会变、什么在抵抗。
///
/// 与 `EntityArcStage`（外部时间线）正交，两个都填，角色才立得住。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CharacterArcPotential {
    pub starting_state: Option<String>,
    pub possible_change: Option<String>,
    pub resistance: Option<String>,
}

/// 题材扩展：类型化枚举 + JSON 兜底（约 80% typed / 20% extra）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CharacterExtension {
    Fantasy(FantasyExtension),
    Modern(ModernExtension),
    SciFi(SciFiExtension),
    Custom(serde_json::Value),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FantasyExtension {
    pub cultivation_level: Option<String>,
    pub realm: Option<String>,
    pub bloodline: Option<String>,
    pub magic_affinity: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModernExtension {
    pub occupation: Option<String>,
    pub income_level: Option<String>,
    pub assets: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SciFiExtension {
    pub cybernetics: Option<Vec<String>>,
    pub augmentation_level: Option<String>,
    pub faction: Option<String>,
}

// ===================== 主结构 =====================

/// Character Profile - 人物的稳定身份约束（通用核心）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CharacterProfile {
    pub id: Uuid,
    pub entity_id: Uuid,
    pub name: Option<String>,
    pub aliases: Vec<String>,
    /// 叙事年龄区间（不是真实年龄）
    pub age: Option<AgeRange>,
    /// 性别（身份约束，不是性格）
    pub gender: Option<Gender>,
    /// 身份（如"边境散修"、"王家家主"）
    pub identity: Option<String>,
    /// 外貌描述
    pub appearance: Option<String>,
    /// 背景故事（导致现在状态的关键经历）
    pub background_origin: Option<String>,
    /// 社会位置（social_status 的抽象）
    pub social_position: Option<SocialPosition>,
    /// 核心性格
    pub core_personality: Option<String>,
    /// 价值观
    pub values: Option<String>,
    /// 角色在故事中的功能位
    pub role_in_story: Option<StoryRole>,
    /// 剧情必要性
    pub narrative_necessity: Option<NarrativeNecessity>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Character State - 人物的当前状态（抽象通用层；题材相关进 Extension）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CharacterState {
    pub id: Uuid,
    pub entity_id: Uuid,
    /// 当前位置
    pub location: Option<String>,
    /// 身体状态（替代 health，可含 limitations）
    pub physical_state: Option<String>,
    /// 心理状态（R2 新增，上一版缺失）
    pub mental_state: Option<String>,
    /// 资源状态（抽象：钱/灵石/能源本质都是资源）
    pub resource_state: Option<String>,
    /// 社会状态（替代 wanted，含 legal_status）
    pub social_state: Option<String>,
    /// 状态标记（如 flags）
    pub flags: Vec<String>,
    /// 自由扩展状态
    pub extra: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Character Trait - 人物的特征（扁平化，去掉 parent_trait_id 层级嵌套）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterTrait {
    pub id: Uuid,
    pub entity_id: Uuid,
    /// 特征类型
    pub trait_type: TraitType,
    /// 特征名称（如"谨慎"）
    pub name: String,
    /// 特征描述
    pub description: Option<String>,
    /// 强度（1-10）
    pub intensity: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 特征类型（保留 Personality/Behavior/Strength/Weakness/Habit/Fear/Value，去掉 Preference）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TraitType {
    Personality,
    Behavior,
    Value,
    Fear,
    Habit,
    Strength,
    Weakness,
}
