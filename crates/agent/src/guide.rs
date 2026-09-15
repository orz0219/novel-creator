//! 引导流程定义（骨架 + 血肉分层）
//!
//! 整个 agent 引导流程的**单一事实源**。
//!
//! ## 设计：骨架 + 血肉分层
//!
//! ```text
//! 骨架（skeleton）— 严格顺序，缺一不可：
//!   premise → world → golden_finger → protagonist → storylines
//!           → beats.chapters → beats.scenes → writing
//!
//! 血肉（flesh）— 可自由顺序，进下一骨架前每类至少 1：
//!   world.map (Location) ┐
//!   world.factions (Faction) ├── 插入在 world 之后
//!   world.items (Item) ┘
//!   characters.supporting ── 插入在 protagonist 之后
//! ```
//!
//! 细纲拆成三步（2026-09 改造）：原先只有一步 `beats` 且它就是流程终点，
//! 造成「AI 建完卷/弧就被告知细纲已就绪，而用户认为细纲才刚开始」。
//! 现在拆为 章表 → 场景 → 正文写作，每一步都有自己的完成判据与下一步。
//!
//! ## 推进规则
//! 1. 用户在前端点按钮 → confirm_step 工具
//! 2. 工具内部校验：当前 step 的 min_complete 满足，**且**如果下一步是骨架步，
//!    所有未完成的血肉 step 至少各有 1 个产物
//! 3. 校验通过 → 推进 current_step
//! 4. 校验失败 → 结构化报告（missing 列表）
//!
//! ## 用户感知
//! - LLM 永远不调 confirm_step
//! - LLM 在 prompt 里看到完整步骤清单 + 当前是哪个 + 哪些是骨架 / 血肉
//!   （由 `crate::prompt::build_system_prompt` 注入，见该函数里的「引导流程全貌」段）
//! - 推进按钮文案 = "确认推进到「下一步」"（不暴露骨架/血肉概念给用户）

use serde::{Deserialize, Serialize};

/// step key 常量（也作为 project.config.current_step 的取值）
///
/// 骨架（skeleton）顺序：
///   premise → world → golden_finger → protagonist → storylines
///           → beats.chapters → beats.scenes → writing
///
/// 血肉（flesh）插入：
///   world.map / world.factions / world.items — 在 world 之后自由
///   characters.supporting — 在 protagonist 之后自由
pub const STEP_PREMISE: &str = "premise";
pub const STEP_WORLD: &str = "world";
pub const STEP_WORLD_MAP: &str = "world.map";
pub const STEP_WORLD_FACTIONS: &str = "world.factions";
pub const STEP_WORLD_ITEMS: &str = "world.items";
pub const STEP_GOLDEN_FINGER: &str = "golden_finger";
pub const STEP_PROTAGONIST: &str = "protagonist";
pub const STEP_SUPPORTING: &str = "characters.supporting";
pub const STEP_STORYLINES: &str = "storylines.main";
pub const STEP_BRANCHES: &str = "storylines.branches";
/// 细纲·章表：卷 → 弧 → 章 三层，每章要有一句话事件
pub const STEP_BEATS_CHAPTERS: &str = "beats.chapters";
/// 细纲·场景：把接下来要写的 3-5 章展开成可写的场景（Scene + attributes）
pub const STEP_BEATS_SCENES: &str = "beats.scenes";
/// 终点：进入正文写作。到这里引导流程结束，之后是常态写作期。
pub const STEP_WRITING: &str = "writing";

/// 历史遗留 step key：拆分前的单步「细纲」。
///
/// 2026-09 拆步后它不再是合法 step，仅用于两件事：
/// 1. 迁移脚本把存量项目的 `current_step` 从 `beats` 改成 `beats.chapters`；
/// 2. `validate_step` 在遇到它时给出**明确的迁移提示**，而不是静默回落。
pub const LEGACY_STEP_BEATS: &str = "beats";

/// 起始 step（创建项目后默认进入的阶段）
pub const INITIAL_STEP: &str = STEP_PREMISE;

/// Step 分组：骨架（必走、强顺序）vs 血肉（可补、但每类至少 1）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepGroup {
    /// 骨架：严格顺序，进下一骨架前本骨架步产物必须就绪
    Skeleton,
    /// 血肉：自由顺序，进下一骨架前所有血肉 step 至少各有 1 个产物
    Flesh,
}

/// 一步引导的完整定义
#[derive(Debug, Clone)]
pub struct GuideStep {
    /// step key（同时作为 current_step 的取值）
    pub key: &'static str,
    /// 中文标题（UI 显示用）
    pub title: &'static str,
    /// step 分组：骨架 / 血肉
    pub group: StepGroup,
    /// 注入到 prompt 的"本步目标 + 落点要求"
    pub prompt_for_step: &'static str,
    /// **"本步产物就绪信号"**：告诉 LLM 满足哪些条件后可以说"可以推进"。
    pub completion_signal: &'static str,
    /// 推进到下一步的最小完整度校验
    pub min_complete: MinComplete,
    /// 下一步 key；空字符串表示当前是最后一步
    pub next: &'static str,
    /// **进入本步时是否需要先过"血肉闸门"**：
    /// 设为 true 的 step（一般是骨架边界）会让 confirm_step 校验"所有血肉 step 至少 1 个产物"。
    /// 比如推进到 storylines 之前需要 world.map / factions / items / supporting 都有至少 1 个。
    pub requires_all_flesh: bool,
}

/// 推进前必须满足的最小完整度
///
/// 用 enum 表达是为了**让校验逻辑集中在此**而不是散落在各分支。
/// 每个变体对应一种 step 的产物校验。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MinComplete {
    /// 永远通过（用于占位 step）
    AlwaysSatisfied,
    /// project.premise 非空
    ProjectPremise,
    /// 世界观骨架：world.description 非空 + 至少 1 条 world rule
    World {
        need_description: bool,
        need_rule_entity: bool,
    },
    /// 血肉：地图——至少 1 个 Location entity
    WorldMap { min_locations: i64 },
    /// 血肉：势力——至少 1 个 Faction entity
    WorldFactions { min_factions: i64 },
    /// 血肉：道具——至少 1 个 Item entity
    WorldItems { min_items: i64 },
    /// 金手指：至少 1 个 GoldenFinger entity + 该 entity 与主角 Character 有 relation
    GoldenFinger {
        need_entity: bool,
        need_relation_to_protagonist: bool,
    },
    /// 骨架：主角——至少 1 个 Character entity
    Protagonist { min_characters: i64 },
    /// 血肉：配角——至少 N 个 Character entity（扣除主角）
    Supporting { min_supporting: i64 },
    /// 故事线：至少 N 条 storyline，其中至少 M 条是主线
    Storylines {
        min_total: i64,
        min_main: i64,
    },
    /// 副线：至少 N 条副线（importance != Main），其中至少 M 条有 parent_id
    Branches {
        min_sub: i64,
        min_attached: i64,
    },
    /// 细纲·章表：卷/弧/章 三层齐了，且每章都有一句话事件。
    ///
    /// 旧的 `Beats`（只要 1 个卷 + 重要线各挂 1 个节点）判定太松：
    /// 实测一个「1 卷 + 5 弧 + 12 个空壳章」的项目就已经通过，
    /// 于是 AI 宣布"细纲已就绪"、用户却认为细纲还没开始。
    BeatsChapters {
        /// 至少 N 个卷节点（node_type='Volume'）
        min_volumes: i64,
        /// 每个弧下至少 N 个章节点（否则这个弧是空壳）
        min_chapters_per_arc: i64,
        /// 是否要求每章都有非空 description（一句话事件）
        require_chapter_description: bool,
        /// 是否要求「importance 为 Main / Important 的线都至少有 1 个节点」
        require_arc_for_important_storylines: bool,
    },
    /// 细纲·场景：把接下来要写的几章展开成可写的场景。
    ///
    /// 必填字段清单见 [`SCENE_REQUIRED_FIELDS`]（模块级常量，校验与 `outline_progress` 工具共用）。
    BeatsScenes {
        /// 至少 N 个场景节点，且这些场景的必填字段齐全
        min_scenes: i64,
    },
}

/// 场景 `attributes` 里必须非空的字段。
///
/// 这四个不是随便挑的：`crates/runtime/src/execution/context_engine.rs` 组装正文生成上下文时，
/// 用 `SceneAttributes` 的这些字段决定「检索哪些角色、哪个地点、哪些关系、哪些近期事件、
/// POV 角色知道什么」。它们空着，正文生成就退化成没有约束的硬编。
pub const SCENE_REQUIRED_FIELDS: &[&str] =
    &["objective", "conflict", "pov_character_id", "location_id"];

/// 校验报告：成功时 `passed=true`；失败时 `passed=false` + `missing` 列表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub passed: bool,
    /// 仅失败时有值；每条是"还差什么"的结构化描述
    #[serde(default)]
    pub missing: Vec<MissingItem>,
    /// 当前 step key（便于前端展示）
    pub current_step: String,
    /// 当前 step 标题
    pub current_title: String,
    /// 下一步 key（passed=true 时有效）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_step: Option<String>,
    /// 下一步标题（passed=true 时有效）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissingItem {
    /// 缺失项类型（agent 据此决定怎么描述）
    pub kind: String,
    /// 用户友好的描述
    pub detail: String,
}

/// 10 步工作流定义表（6 骨架 + 4 血肉）
///
/// ## 顺序流
/// 骨架（严格顺序）：
///   premise → world → golden_finger → protagonist → storylines → beats
/// 血肉（自由顺序，在 world 之后、protagonist 之后插入）：
///   world.map / world.factions / world.items — 插入在 world 之后
///   characters.supporting — 插入在 protagonist 之后
///
/// ## 推进规则
/// - 每个 step 校验自己的 `min_complete`
/// - 当 `requires_all_flesh=true` 时，额外校验所有血肉 step 至少 1 个产物
pub const STEPS: &[GuideStep] = &[
    // ===== 骨架 1/5：premise =====
    GuideStep {
        key: STEP_PREMISE,
        title: "故事脑洞",
        group: StepGroup::Skeleton,
        prompt_for_step: "本步：聊故事脑洞（premise）。脑洞是模糊的一句话核心反常设定，\
                          落点：project.premise。\n\
                          你的工作方式：\n\
                          - 倾听为主，让用户先讲他脑子里那个模糊的想法；\n\
                          - 追问 1-2 个关键问题帮用户把脑洞磨清晰；\n\
                          - **不要主动调 update_project 落库**；\n\
                          - 当用户表达了相对完整的脑洞，主动总结为一句 premise 复述给用户，\
                            等用户在前端点'确认推进'按钮才推进到下一步。",
        completion_signal: "本步产物就绪条件：\n\
                           - 用户已描述一个核心反常设定；\n\
                           - 复述为一句 premise 后用户明确说'对' / '可以' / '就这意思了'；\n\
                           - 你已成功调 update_project 工具把这句话写入 project.premise。\n\
                           三者都满足 → 告诉用户'脑洞已写入项目，可以推进到世界观了。请点下方\"确认推进\"按钮。'",
        min_complete: MinComplete::ProjectPremise,
        next: STEP_WORLD,
        requires_all_flesh: false,
    },

    // ===== 骨架 2/5：world =====
    GuideStep {
        key: STEP_WORLD,
        title: "世界观",
        group: StepGroup::Skeleton,
        prompt_for_step: "本步：基于已定的脑洞，聊世界观的**骨架**。落点：\n\
                          - world.description 非空（用 update_main_world 工具写）；\n\
                          - 至少 1 条 world rule（用 create_rule 工具建）。\n\
                          **地图/势力/道具**在后续血肉 step 单独建，本步不要求。\n\
                          建议先问用户时代/地点，再聊大规则。",
        completion_signal: "本步产物就绪条件：\n\
                           - world.description 已经被 update_main_world 工具写入；\n\
                           - 至少 1 条 world rule 已经用 create_rule 工具建好。\n\
                           两者都满足 → 告诉用户'世界观骨架已就绪，可以推进到金手指了。\
                           后续可以补地图/势力/道具（血肉 step）。请点下方\"确认推进\"按钮。'",
        min_complete: MinComplete::World {
            need_description: true,
            need_rule_entity: true,
        },
        next: STEP_GOLDEN_FINGER,
        requires_all_flesh: false,
    },

    // ===== 血肉 1/4：world.map =====
    GuideStep {
        key: STEP_WORLD_MAP,
        title: "地图",
        group: StepGroup::Flesh,
        prompt_for_step: "本步：聊**地图 / 地点**。落点：至少 1 个 Location entity（entity_type='Location'）。\n\
                          用 create_location 工具建。建议：1 个主城 + 1-2 个关键地点；\n\
                          可选：给主角一个常驻地点作为'据点'。\n\
                          **建完地点后紧接着用 update_location_profile 补全它的设计档案**\n\
                          （地点类型 / 规模 / 气候 / 纪元 / 可达性 / 人口 / 地理 / 外貌 / 经济 / 规则 / 历史 / 叙事用途），\n\
                          只填你与用户已经聊定的字段，不要凭空编造——档案缺失会让该地点详情面板一片空白。\n\
                          若该地点的**叙事角色**随剧情变化（主角藏身处→教团总部→两军战场），用 arc_stages 按阶段写 role / function / screen_weight / status；\n\
                          被烧毁、易主这类物理变化属于剧情事件，不要写进阶段。\n\
                          注意：这是**血肉 step**，用户可自由顺序来建（不强制先聊这个）。\n\
                          推进到下一骨架步前，每类血肉至少 1 个。",
        completion_signal: "本步产物就绪条件：至少 1 个 Location entity 已建。\
                           满足 → 告诉用户'地图已就绪。请点下方\"确认推进\"按钮进入下一血肉 step。'",
        min_complete: MinComplete::WorldMap { min_locations: 1 },
        next: STEP_GOLDEN_FINGER,
        requires_all_flesh: false,
    },

    // ===== 血肉 2/4：world.factions =====
    GuideStep {
        key: STEP_WORLD_FACTIONS,
        title: "势力",
        group: StepGroup::Flesh,
        prompt_for_step: "本步：聊**势力 / 组织**。落点：至少 1 个 Faction entity。\n\
                          用 create_faction 工具建。建议：1 个敌对势力 + 1 个友方势力。\n\
                          **建完势力后紧接着用 update_faction_profile 补全它的设计档案**\n\
                          （目标 goals / 领袖 leader / 价值观 values / 资源 resources / 领地 territory /\n\
                          成员 members / 敌人 enemies / 盟友 allies / 内部矛盾 internal_conflicts /\n\
                          秘密 secrets / 行事风格 modus_operandi），\n\
                          只填你与用户已经聊定的字段，不要凭空编造——档案缺失会让势力详情面板一片空白。\n\
                          势力的实力/地盘/盟友会随剧情变化，用 arc_stages 按阶段记录 role（在故事里的角色）/ goal（该阶段目标）/ status（该阶段多强、占哪、跟谁结盟）；\n\
                          **血肉 step**，用户可自由顺序。",
        completion_signal: "本步产物就绪条件：至少 1 个 Faction entity 已建。\
                           满足 → 告诉用户'势力已就绪。请点下方\"确认推进\"按钮。'",
        min_complete: MinComplete::WorldFactions { min_factions: 1 },
        next: STEP_GOLDEN_FINGER,
        requires_all_flesh: false,
    },

    // ===== 血肉 3/4：world.items =====
    GuideStep {
        key: STEP_WORLD_ITEMS,
        title: "道具",
        group: StepGroup::Flesh,
        prompt_for_step: "本步：聊**道具 / 物品**。落点：至少 1 个 Item entity。\n\
                          用 create_character (entity_type='Item') 工具建。\n\
                          注意：金手指是独立 entity_type='golden_finger'，**不是** Item；\
                          这里指主角的装备、信物、消耗品等。\n\
                          **血肉 step**，用户可自由顺序。",
        completion_signal: "本步产物就绪条件：至少 1 个 Item entity 已建。\
                           满足 → 告诉用户'道具已就绪。请点下方\"确认推进\"按钮。'",
        min_complete: MinComplete::WorldItems { min_items: 1 },
        next: STEP_GOLDEN_FINGER,
        requires_all_flesh: false,
    },

    // ===== 骨架 3/5：golden_finger =====
    GuideStep {
        key: STEP_GOLDEN_FINGER,
        title: "金手指",
        group: StepGroup::Skeleton,
        prompt_for_step: "本步：聊金手指。\n\
                          **金手指是独立 entity（entity_type='golden_finger'）**，\
                          通过 create_relation 与主角 Character 相连（relation type='possesses'）。\n\
                          落库步骤（三步都要做，缺一步金手指在界面上就是残的）：\n\
                          1) **用 create_entity 并把 entity_type 传 'golden_finger'** 建金手指\n\
                             （不要用 create_character——那会把类型固化成 Character，界面上就没有金手指了）；\n\
                          2) 用 create_relation 把主角与金手指连起来；\n\
                          3) **用 update_golden_finger 填写结构化档案**——界面的金手指面板完全依赖这些字段，\n\
                             不填则面板一片空白、用户看不出这个金手指到底有什么用。要填的是：\n\
                             gf_type（类型）、one_liner（一句话作用）、origin（来源）、\n\
                             abilities（核心机制，逐条写清 effect 效果 / trigger 触发条件 / limit 该机制的边界）、\n\
                             side_effect（副作用；作者设定为「无」也要显式写「无」，不能留空）、\n\
                             constraints（硬约束：什么时候会失效、必须满足什么前提——这是后续写作最容易崩的地方）、\n\
                             growth_stages（初期 / 中期 / 后期分别能做什么，按故事时间顺序）。\n\
                          这些内容必须和用户逐条聊定，不要凭空编造。",
        completion_signal: "本步产物就绪条件：\n\
                           - 1 个 golden_finger entity 已建；\n\
                           - 该金手指已通过 create_relation 与主角 Character 相连。\n\
                           两者都满足 → 告诉用户'金手指已就绪，可以推进到主角了。请点下方\"确认推进\"按钮。'",
        min_complete: MinComplete::GoldenFinger {
            need_entity: true,
            need_relation_to_protagonist: true,
        },
        next: STEP_PROTAGONIST,
        requires_all_flesh: false,
    },

    // ===== 骨架 4/5：protagonist =====
    GuideStep {
        key: STEP_PROTAGONIST,
        title: "主角",
        group: StepGroup::Skeleton,
        prompt_for_step: "本步：聊**主角**（一个 Character entity）。\n\
                          落库步骤（两步都要做）：\n\
                          1) 用 create_character 工具建主角（name + description + 人物小传）；\n\
                          2) **用 update_character_profile 填写结构化档案**——不填的话，\
                             人物页面的「角色设定」就是一张空表，等于设定没落地。逐项填：\n\
                             真名 / 别名 aliases / 年龄段 age_range / 性别 gender / 身份 identity /\n\
                             外貌 appearance / 背景来历 background_origin / 核心性格 core_personality /\n\
                             价值观 values / 故事功能位 role_in_story / 社会地位 social_position_rank。\n\
                             若这个角色会跨阶段变化（前期只是背景板、中期成为合伙人、后期成重头戏），\
                             用 arc_stages 列出每个阶段的 stage / order / role / screen_weight / goal / entry_trigger；\
                             不跨阶段可以不传这个字段。\n\
                             age_range、gender、role_in_story 可直接传中文（如「青年」「男」「主角」），\n\
                             也可以传规范值，两种写法都会正确落库，无法识别的写法才会报错。\n\
                             只填与用户已经聊定的内容，没聊到的字段不要传。\n\
                          主角必须在前一步（golden_finger）已与金手指有 'possesses' relation。\n\
                          配角在后续血肉 step 单独建。",
        completion_signal: "本步产物就绪条件：至少 1 个 Character entity（主角）已建。\
                           满足 → 告诉用户'主角已就绪，可以推进到故事线了。\
                           后续可以补配角（血肉 step）。请点下方\"确认推进\"按钮。'",
        min_complete: MinComplete::Protagonist { min_characters: 1 },
        next: STEP_STORYLINES,
        requires_all_flesh: false,
    },

    // ===== 血肉 4/4：characters.supporting =====
    GuideStep {
        key: STEP_SUPPORTING,
        title: "配角",
        group: StepGroup::Flesh,
        prompt_for_step: "本步：聊**关键配角**。落点：至少 1 个 Character entity（扣除主角）。\n\
                          用 create_character 工具建。建议 2-3 个：\n                          - 1 个盟友（ally）；\n                          - 1 个对手（rival）；\n                          - 可选：1 个导师 / 引路人。\n\
                          **建完每个配角后紧接着用 update_character_profile 补全档案**\n\
                          （真名 / 别名 / 年龄段 / 性别 / 身份 / 外貌 / 背景来历 / 核心性格 / 价值观 /\n\
                          故事功能位 / 社会地位），只填已聊定的内容，不要凭空编造；\n\
                          若某个配角只出现一段、不跨阶段，就不必填 arc_stages；\
                          确实跨阶段变化的，再用 arc_stages 标出他何时上场、戏份多大。\n\
                          age_range、gender、role_in_story 可直接传中文（如「青年」「男」「盟友」），\n\
                          配角与主角通过 create_relation 建立关系。\n\
                          **血肉 step**，用户可自由顺序。",
        completion_signal: "本步产物就绪条件：至少有 1 个非主角的 Character entity 已建。\
                           满足 → 告诉用户'配角已就绪。请点下方\"确认推进\"按钮。'",
        min_complete: MinComplete::Supporting { min_supporting: 1 },
        next: STEP_STORYLINES,
        requires_all_flesh: false,
    },

    // ===== 骨架 5/5：storylines.main（先有"树干"） =====
    GuideStep {
        key: STEP_STORYLINES,
        title: "故事线",
        group: StepGroup::Skeleton,
        prompt_for_step: "本步：先聊故事的主线。\n\
                          主线 = 整本书讲的一个核心任务/目标，例如：\n\
                          - '主角团屠龙' / '追凶找出真凶' / '打破时间循环'。\n\
                          落点：1 条 importance='Main' 的 storyline（用 create_storyline 工具）。\n\
                          主线必须是明线（tone='light'）+ 暴露给读者（visibility='visible'）。\n\
                          **重要：进入本步前**（即从世界 / 主角 / 配角 step 推进到本步时）\n\
                          需保证所有血肉 step（地图/势力/道具/配角）都至少 1 个产物。",
        completion_signal: "本步产物就绪条件：\n\
                           - 至少 1 条 importance='Main' 的主线已用 create_storyline 工具建好；\n\
                           - 且**所有血肉 step**（地图/势力/道具/配角）都已至少 1 个产物。\n\
                           满足 → 告诉用户'主线已定，可以补副线了。请点下方\"确认推进\"按钮。'",
        min_complete: MinComplete::Storylines {
            // 主线步：只要求有 1 条 Main（min_total=0 让主线决定）
            min_total: 0,
            min_main: 1,
        },
        next: STEP_BRANCHES,
        requires_all_flesh: true, // ← 关键：进入本步需所有血肉都有
    },

    // ===== 血肉：storylines.branches（"树枝"，挂到主线上） =====
    GuideStep {
        key: STEP_BRANCHES,
        title: "副线",
        group: StepGroup::Flesh,
        prompt_for_step: "本步：聊**副线**（branch）。每条副线都是主线的'分支'：\n\
                          - **明线副线**（tone=light）：辅助故事，揭示配角/世界观/情感线；\n\
                          - **暗线副线**（tone=dark + visibility=hidden）：伏笔/钩子，\n\
                            揭示幕后真相（如'凶手的真实身份'），前期对读者隐藏。\n\
                          \n\
                          每条副线必须**挂到主线**（或挂到另一条副线）—— 用 create_storyline 时传 parent_id。\n\
                          \n\
                          建议至少 1 条明线副线 + 可选 1 条暗线副线（伏笔/钩子）。\n\
                          例子（时间循环脑洞）：\n\
                          - 明线副线：'老张为什么死？'（揭开过去）\n\
                          - 暗线副线：'循环的真正源头'（前期隐藏）\n\
                          \n\
                          这是血肉 step——可自由顺序；如果先做了主角/配角也可以先回来补。",
        completion_signal: "本步产物就绪条件：\n\
                           - 至少 1 条 importance != 'Main' 的副线；\n\
                           - 至少 1 条副线有 parent_id（挂到主线/其他副线）。\n\
                           满足 → 告诉用户'副线已就绪，可以推进到细纲了。请点下方\"确认推进\"按钮。'",
        min_complete: MinComplete::Branches {
            min_sub: 1,         // 至少 1 条副线（不论明暗）
            min_attached: 1,    // 至少 1 条副线挂到主线/其他副线
        },
        next: STEP_BEATS_CHAPTERS,
        requires_all_flesh: false,
    },

    // ===== 骨架 6/8：细纲·章表 =====
    GuideStep {
        key: STEP_BEATS_CHAPTERS,
        title: "细纲·章表",
        group: StepGroup::Skeleton,
        prompt_for_step: "本步：把故事线落成「卷 → 弧 → 章」三层节点树，并给每一章写清一句话事件。\n\
                          落点：narrative_node（bulk_create_nodes 批量建树，create_node 单个补，\
                          revise_node 移动 / 改文案）。\n\
                          你的工作方式（**按弧推进：一次只做一个弧，做完停下来问用户**）：\n\
                          - 先 list_nodes（roots_only=true）看已有的卷，再逐层下钻（parent_id），\
                            不要重复建已经存在的结构；\n\
                          - 一个弧一轮：确认/建立这个弧，然后把它下面的章建出来；\n\
                          - 每章必须同时给两样东西：title（形如「第 4 章 · 赵奎追至」）与 description\n\
                            （一句话说清「谁做了什么、结果如何」——这是后面写正文的唯一依据）；\n\
                          - 每个节点用 storyline_id + arc_stage 认领它服务的那条线与那个阶段\n\
                            （阶段名要与该故事线 arc_stages 里的 stage 一致），\n\
                            一个节点服务多条线时用 stage_refs；\n\
                          - 结构挂载：participant_entity_ids（在场角色）/ location_id / item_ids；\n\
                            规模用 estimated_chapters / estimated_words，故事内时间用 story_time；\n\
                          - **建错位置不要 retire 重建**：用 revise_node 改 parent_id / sort_order 移动即可，\n\
                            节点 id 不变，伏笔的埋点与回收点因此不会脱钩；\n\
                          - 一个弧做完，把「这几章怎么走」用三五句话复述给用户，问他是否调整，\
                            确认后再做下一个弧。\n\
                          反例（不要这样做）：\n\
                          - 一次 bulk_create_nodes 把全书八十章全铺完——用户没法逐步校准，\
                            而且 description 必然写成套话；\n\
                          - 只建章、不写 description——那是空壳章，本步不算完成；\n\
                          - 层级错位：卷是顶层、弧挂卷下、章挂弧下，不要平铺。",
        completion_signal: "本步产物就绪条件：\n\
                           - 至少 1 个卷节点（node_type='Volume'）；\n\
                           - 每个弧下面至少有 2 个章节点（没有空壳弧）；\n\
                           - 每一章都有非空的 description（一句话事件，没有空壳章）；\n\
                           - 每条重要故事线（importance 为 Main / Important）都至少有 1 个节点挂到它。\n\
                           满足 → 告诉用户'章表已就绪，接下来把要写的几章展开成场景'，\
                           并提示他点下方的推进按钮。",
        min_complete: MinComplete::BeatsChapters {
            min_volumes: 1,
            min_chapters_per_arc: 2,
            require_chapter_description: true,
            require_arc_for_important_storylines: true,
        },
        next: STEP_BEATS_SCENES,
        requires_all_flesh: false,
    },

    // ===== 骨架 7/8：细纲·场景 =====
    GuideStep {
        key: STEP_BEATS_SCENES,
        title: "细纲·场景",
        group: StepGroup::Skeleton,
        prompt_for_step: "本步：把「接下来要写的 3-5 章」展开成可写的场景（Scene）。\n\
                          **不要全书一次展开**——细纲是滚动维护的，永远只保证前方几章是细的。\n\
                          落点：create_node(node_type='Scene', parent_id=<章 id>)，\
                          关键内容填在 attributes 里。\n\
                          你的工作方式（**一章一轮**）：\n\
                          - 先用 list_nodes 找出「有 description 但下面还没有场景」的章，从最靠前的开始；\n\
                          - 把这一章拆成 2-4 个场景。场景 = 一个时空连续单元：换了地点或时间就是新场景；\n\
                          - 每个场景的 attributes 至少要有这四个字段（正文生成引擎真正会读的）：\n\
                            objective（本场要达成什么，一句话、可验证）、conflict（什么在挡：人 / 规则 / 时间 / 自身）、\n\
                            pov_character_id（视角人物 entity id，必填——它决定正文里「谁知道什么」）、\n\
                            location_id（发生地点 entity id）；\n\
                          - 建议一并补上（有就填，没想清楚不要编）：\n\
                            time（故事内时间，如「第三天黄昏」）、emotional_goal / information_goal、\n\
                            required_events（必须发生的事，生成时作为硬约束）、\n\
                            forbidden_events（禁止发生的事，防止擅自发便当或提前泄露真相）、\n\
                            characters_present（在场角色 entity id 数组）；\n\
                          - 一章的场景建完，用一句话向用户复述「这一章怎么打」，问他要不要调整；\n\
                          - 用户确认后再做下一章。\n\
                          反例（不要这样做）：\n\
                          - 一次把几十章的场景全铺开——用户没法逐章校准，字段也会填成空话；\n\
                          - 场景只填 title 不填 attributes——这等于没有细纲，正文生成读不到任何约束；\n\
                          - 擅自新增/删除用户已经定好的人物与地点——需要新角色时先问用户。",
        completion_signal: "本步产物就绪条件：\n\
                           - 至少 3 个场景节点（node_type='Scene'）挂在章的下面；\n\
                           - 每个场景的 attributes 里 objective / conflict / pov_character_id / location_id\n\
                             四个字段都非空（这是正文生成引擎真正会读的字段）。\n\
                           满足 → 告诉用户'场景已就绪，可以进入正文写作了'，并提示他点下方推进按钮。",
        min_complete: MinComplete::BeatsScenes { min_scenes: 3 },
        next: STEP_WRITING,
        requires_all_flesh: false,
    },

    // ===== 骨架 8/8：正文（终点站） =====
    GuideStep {
        key: STEP_WRITING,
        title: "正文",
        group: StepGroup::Skeleton,
        prompt_for_step: "本步：引导流程的最后一站——陪用户把场景写成正文。\n\
                          正文本身在「写作页」产出（选场景 → AI 生成 → 人工修改），\
                          你在这里的职责是四件事：\n\
                          - 指明从哪开始：用 outline_progress 看哪些场景的细纲已经齐了，建议从最靠前的写；\n\
                          - 一次一场：写完一场先让用户看效果，再决定继续写还是返修，不要连着生成十场；\n\
                          - 保持「前方 3-5 章有完整细纲」：写到某个弧的最后一章时，提醒用户先补下一个弧的章表与场景；\n\
                          - 守连贯性：新写的内容与已定设定 / 已埋伏笔冲突时，先指出来，再讨论怎么改。\n\
                          不要再问用户「要不要点推进按钮」——这里已经没有下一步了。",
        completion_signal: "本步是引导流程的终点，**没有下一步，也不要再提「确认推进」按钮**。\n\
                           本步没有产物门槛：进入后即为常态写作期。\n\
                           当所有章节点 status 都是 Completed 时，全书完稿。",
        min_complete: MinComplete::AlwaysSatisfied,
        next: "",
        requires_all_flesh: false,
    },
];

/// 按 key 查找 step 定义；找不到返回 None
pub fn find_step(key: &str) -> Option<&'static GuideStep> {
    STEPS.iter().find(|s| s.key == key)
}

/// 给定当前 step key，返回下一步 key（最后一步返回 None）
pub fn next_step_key(key: &str) -> Option<&'static str> {
    let step = find_step(key)?;
    if step.next.is_empty() {
        None
    } else {
        Some(step.next)
    }
}

/// 给定当前 step key 与该 step 校验所需的"当前状态"，产出推进报告
///
/// 这是 confirm_step 工具的**核心逻辑**：由工具实现把 DB 状态打包成
/// [`MinCompleteSnapshot`] 后传入本函数，本函数只做布尔校验与"还差什么"诊断。
pub fn validate_step(key: &str, snapshot: &MinCompleteSnapshot) -> ValidationReport {
    let step = match find_step(key) {
        Some(s) => s,
        None => {
            // 遗留 key 必须给出可执行的迁移提示，而不是笼统的「未知阶段」：
            // 拆分前的单步 beats 是绝大多数存量项目会停在的地方（迁移 042 会改写它，
            // 但代码与数据版本不一致时这里要能自己说清楚发生了什么）。
            let detail = if key == LEGACY_STEP_BEATS {
                format!(
                    "当前阶段 key 是拆分前的旧值 `{}`，它已被拆成 `{}`（章表）与 `{}`（场景）两步。\
                     请把 project.config.current_step 迁移为 `{}`（迁移文件 042_narrative_guide_beats_split.sql），\
                     或由用户在界面上重置引导阶段。",
                    LEGACY_STEP_BEATS, STEP_BEATS_CHAPTERS, STEP_BEATS_SCENES, STEP_BEATS_CHAPTERS
                )
            } else {
                format!("未知的引导阶段: {}", key)
            };
            return ValidationReport {
                passed: false,
                missing: vec![MissingItem {
                    kind: if key == LEGACY_STEP_BEATS {
                        "legacy_step_key".into()
                    } else {
                        "unknown_step".into()
                    },
                    detail,
                }],
                current_step: key.to_string(),
                current_title: key.to_string(),
                next_step: None,
                next_title: None,
            };
        }
    };

    let mut missing = Vec::new();

    // 1) 本 step 的 min_complete 校验
    match &step.min_complete {
        MinComplete::AlwaysSatisfied => {}
        MinComplete::ProjectPremise => {
            if snapshot.project_premise.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true) {
                missing.push(MissingItem {
                    kind: "project_premise".into(),
                    detail: "故事脑洞（premise）尚未落库".into(),
                });
            }
        }
        MinComplete::World { need_description, need_rule_entity } => {
            if *need_description
                && snapshot
                    .world_description
                    .as_ref()
                    .map(|s| s.trim().is_empty())
                    .unwrap_or(true)
            {
                missing.push(MissingItem {
                    kind: "world_description".into(),
                    detail: "世界观描述（world.description）尚未填写".into(),
                });
            }
            if *need_rule_entity && snapshot.world_rule_entity_count == 0 {
                missing.push(MissingItem {
                    kind: "world_rule_entity".into(),
                    detail: "至少需要 1 条 world rule".into(),
                });
            }
        }
        MinComplete::WorldMap { min_locations } => {
            if snapshot.location_entity_count < *min_locations {
                missing.push(MissingItem {
                    kind: "location_entity".into(),
                    detail: format!("至少需要 {} 个 Location entity（地图/地点）", min_locations),
                });
            }
        }
        MinComplete::WorldFactions { min_factions } => {
            if snapshot.faction_entity_count < *min_factions {
                missing.push(MissingItem {
                    kind: "faction_entity".into(),
                    detail: format!("至少需要 {} 个 Faction entity（势力）", min_factions),
                });
            }
        }
        MinComplete::WorldItems { min_items } => {
            if snapshot.item_entity_count < *min_items {
                missing.push(MissingItem {
                    kind: "item_entity".into(),
                    detail: format!("至少需要 {} 个 Item entity（道具）", min_items),
                });
            }
        }
        MinComplete::GoldenFinger { need_entity, need_relation_to_protagonist } => {
            if *need_entity && snapshot.golden_finger_entity_count == 0 {
                missing.push(MissingItem {
                    kind: "golden_finger_entity".into(),
                    detail: "至少需要 1 个 golden_finger entity".into(),
                });
            }
            if *need_relation_to_protagonist && !snapshot.golden_finger_has_relation_to_protagonist {
                missing.push(MissingItem {
                    kind: "golden_finger_relation".into(),
                    detail: "金手指与主角之间尚未建立 relation（possesses）".into(),
                });
            }
        }
        MinComplete::Protagonist { min_characters } => {
            if snapshot.character_entity_count < *min_characters {
                missing.push(MissingItem {
                    kind: "protagonist".into(),
                    detail: format!("至少需要 {} 个 Character entity（主角）", min_characters),
                });
            }
        }
        MinComplete::Supporting { min_supporting } => {
            let supporting = (snapshot.character_entity_count - 1).max(0);
            if supporting < *min_supporting {
                missing.push(MissingItem {
                    kind: "supporting_character".into(),
                    detail: format!(
                        "至少需要 {} 个非主角的 Character entity（配角），当前 {} 个",
                        min_supporting, supporting
                    ),
                });
            }
        }
        MinComplete::Storylines { min_total, min_main } => {
            if snapshot.storyline_total_count < *min_total {
                missing.push(MissingItem {
                    kind: "storyline_total".into(),
                    detail: format!(
                        "至少需要 {} 条故事线，当前 {} 条",
                        min_total, snapshot.storyline_total_count
                    ),
                });
            }
            if snapshot.main_storyline_count < *min_main {
                missing.push(MissingItem {
                    kind: "main_storyline".into(),
                    detail: format!(
                        "至少需要 {} 条主故事线（importance='Main'）",
                        min_main
                    ),
                });
            }
        }
        MinComplete::Branches { min_sub, min_attached } => {
            if snapshot.sub_storyline_count < *min_sub {
                missing.push(MissingItem {
                    kind: "sub_storyline".into(),
                    detail: format!(
                        "至少需要 {} 条副线（importance != 'Main'），当前 {} 条",
                        min_sub, snapshot.sub_storyline_count
                    ),
                });
            }
            if snapshot.attached_storyline_count < *min_attached {
                missing.push(MissingItem {
                    kind: "attached_storyline".into(),
                    detail: format!(
                        "至少需要 {} 条副线挂到主线/其他副线（parent_id 非空），当前 {} 条",
                        min_attached, snapshot.attached_storyline_count
                    ),
                });
            }
        }
        MinComplete::BeatsChapters {
            min_volumes,
            min_chapters_per_arc,
            require_chapter_description,
            require_arc_for_important_storylines,
        } => {
            if snapshot.volume_node_count < *min_volumes {
                missing.push(MissingItem {
                    kind: "volume_node".into(),
                    detail: format!(
                        "细纲至少需要 {} 个卷节点（node_type='Volume'），当前 {} 个",
                        min_volumes, snapshot.volume_node_count
                    ),
                });
            }
            if snapshot.arcs_without_chapter > 0 {
                let needed = (*min_chapters_per_arc).max(1);
                missing.push(MissingItem {
                    kind: "arc_without_chapter".into(),
                    detail: format!(
                        "还有 {} 个弧下面一个章都没有（每个弧至少要 {} 个章节点）：\
                         弧不能是空壳，用 create_node(parent_id=弧 id, node_type='Chapter') 补章",
                        snapshot.arcs_without_chapter, needed
                    ),
                });
            }
            if *require_chapter_description && snapshot.chapters_without_description > 0 {
                missing.push(MissingItem {
                    kind: "chapter_without_description".into(),
                    detail: format!(
                        "还有 {} 章没有一句话事件（description 为空）：\
                         每章都要用 revise_node 写明「谁做了什么、结果如何」",
                        snapshot.chapters_without_description
                    ),
                });
            }
            if *require_arc_for_important_storylines
                && snapshot.important_storylines_without_node > 0
            {
                missing.push(MissingItem {
                    kind: "storyline_without_node".into(),
                    detail: format!(
                        "还有 {} 条重要故事线（Main / Important）没有任何节点挂到它：\
                         每条重要线至少要有 1 个节点（通常是一个弧或章），\
                         用 create_node 的 storyline_id 认领",
                        snapshot.important_storylines_without_node
                    ),
                });
            }
        }
        MinComplete::BeatsScenes { min_scenes } => {
            if snapshot.scene_node_count < *min_scenes {
                missing.push(MissingItem {
                    kind: "scene_node".into(),
                    detail: format!(
                        "细纲至少需要 {} 个场景节点（node_type='Scene'），当前 {} 个：\
                         把接下来要写的几章展开成场景，正文是按场景生成的",
                        min_scenes, snapshot.scene_node_count
                    ),
                });
            }
            if snapshot.scene_node_count > 0 && snapshot.scenes_missing_required_fields > 0 {
                missing.push(MissingItem {
                    kind: "scene_missing_fields".into(),
                    detail: format!(
                        "还有 {} 个场景缺少必填属性（{}）：\
                         这些字段是正文生成引擎真正会读的，缺了正文就只能硬编",
                        snapshot.scenes_missing_required_fields,
                        SCENE_REQUIRED_FIELDS.join(" / ")
                    ),
                });
            }
        }
    }

    // 2) 血肉闸门（如果本 step 标记了 requires_all_flesh=true）
    if step.requires_all_flesh {
        if snapshot.location_entity_count == 0 {
            missing.push(MissingItem {
                kind: "flesh_map".into(),
                detail: "血肉 step 缺：至少需要 1 个 Location entity（地图/地点）".into(),
            });
        }
        if snapshot.faction_entity_count == 0 {
            missing.push(MissingItem {
                kind: "flesh_factions".into(),
                detail: "血肉 step 缺：至少需要 1 个 Faction entity（势力）".into(),
            });
        }
        if snapshot.item_entity_count == 0 {
            missing.push(MissingItem {
                kind: "flesh_items".into(),
                detail: "血肉 step 缺：至少需要 1 个 Item entity（道具）".into(),
            });
        }
        let supporting = (snapshot.character_entity_count - 1).max(0);
        if supporting == 0 {
            missing.push(MissingItem {
                kind: "flesh_supporting".into(),
                detail: "血肉 step 缺：至少需要 1 个非主角的 Character entity（配角）".into(),
            });
        }
    }

    let passed = missing.is_empty();
    let (next_step, next_title) = if passed {
        match next_step_key(key) {
            Some(next_key) => {
                let t = find_step(next_key).map(|s| s.title).unwrap_or(next_key);
                (Some(next_key.to_string()), Some(t.to_string()))
            }
            None => (None, None),
        }
    } else {
        (None, None)
    };

    ValidationReport {
        passed,
        missing,
        current_step: step.key.to_string(),
        current_title: step.title.to_string(),
        next_step,
        next_title,
    }
}

/// DB 当前状态快照（由 confirm_step 工具从 DB 查询后组装）
///
/// 这是**校验与 ORM 的解耦点**：validate_step 只依赖这个结构体，不关心
/// 它怎么从 DB 取到。工具实现负责把 sqlx 查询结果映射进来。
#[derive(Debug, Clone, Default)]
pub struct MinCompleteSnapshot {
    pub project_premise: Option<String>,
    pub world_description: Option<String>,
    pub world_rule_entity_count: i64,
    pub location_entity_count: i64,
    pub faction_entity_count: i64,
    pub item_entity_count: i64,
    pub golden_finger_entity_count: i64,
    pub golden_finger_has_relation_to_protagonist: bool,
    pub character_entity_count: i64, // 主角 + 配角总
    pub main_storyline_count: i64,
    pub storyline_total_count: i64,
    pub sub_storyline_count: i64,        // 副线数（importance != Main）
    pub attached_storyline_count: i64,   // 副线中有 parent_id 挂载的条数
    /// 细纲：卷节点数（node_type = 'Volume'）
    pub volume_node_count: i64,
    /// 细纲：还没有任何节点挂到它的重要故事线（importance 为 Main / Important）条数
    pub important_storylines_without_node: i64,
    /// 细纲：章节点总数（node_type = 'Chapter'）
    pub chapter_node_count: i64,
    /// 细纲：没有任何章节点的弧数（空壳弧）
    pub arcs_without_chapter: i64,
    /// 细纲：description 为空的章数（空壳章）
    pub chapters_without_description: i64,
    /// 细纲：场景节点总数（node_type = 'Scene'）
    pub scene_node_count: i64,
    /// 细纲：attributes 里缺必填字段的场景数（与要求的字段无关，由校验时按字段逐项判断）
    pub scenes_missing_required_fields: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_snapshot() -> MinCompleteSnapshot {
        MinCompleteSnapshot::default()
    }

    #[test]
    fn test_initial_step_is_premise() {
        assert_eq!(INITIAL_STEP, STEP_PREMISE);
    }

    #[test]
    fn test_steps_have_valid_next() {
        // next 必须指向 STEPS 里存在的 key（最后一步 next 是空字符串）
        for s in STEPS {
            if s.next.is_empty() {
                continue;
            }
            assert!(
                find_step(s.next).is_some(),
                "step '{}' points to non-existent next '{}'",
                s.key,
                s.next
            );
        }
    }

    #[test]
    fn test_all_steps_have_completion_signal() {
        for s in STEPS {
            assert!(
                !s.completion_signal.trim().is_empty(),
                "step '{}' missing completion_signal",
                s.key
            );
        }
    }

    #[test]
    fn test_next_step_key_last_returns_none() {
        // 拆步后终点是 writing（原先终点是单步的 beats）
        assert_eq!(next_step_key(STEP_WRITING), None);
        assert_eq!(next_step_key(STEP_BEATS_SCENES), Some(STEP_WRITING));
        assert_eq!(next_step_key(STEP_BEATS_CHAPTERS), Some(STEP_BEATS_SCENES));
        assert_eq!(next_step_key(STEP_BRANCHES), Some(STEP_BEATS_CHAPTERS));
    }

    #[test]
    fn test_next_step_key_middle() {
        assert_eq!(next_step_key(STEP_PREMISE), Some(STEP_WORLD));
        assert_eq!(next_step_key(STEP_GOLDEN_FINGER), Some(STEP_PROTAGONIST));
        // 血肉 step 都可以推进到下一个骨架步
        assert_eq!(next_step_key(STEP_WORLD_MAP), Some(STEP_GOLDEN_FINGER));
        assert_eq!(next_step_key(STEP_SUPPORTING), Some(STEP_STORYLINES));
    }

    #[test]
    fn test_unknown_step_returns_error() {
        let r = validate_step("nonsense", &empty_snapshot());
        assert!(!r.passed);
        assert_eq!(r.missing.len(), 1);
        assert_eq!(r.missing[0].kind, "unknown_step");
    }

    #[test]
    fn test_premise_step_requires_premise() {
        let r = validate_step(STEP_PREMISE, &empty_snapshot());
        assert!(!r.passed);
        assert!(r.missing.iter().any(|m| m.kind == "project_premise"));

        let mut s = empty_snapshot();
        s.project_premise = Some("我捡了一块钱花不完".into());
        let r = validate_step(STEP_PREMISE, &s);
        assert!(r.passed);
        assert_eq!(r.next_step.as_deref(), Some(STEP_WORLD));
    }

    #[test]
    fn test_world_step_requires_two_things() {
        // 新版 world 骨架只要求 description + rule（location 拆到血肉 step）
        let r = validate_step(STEP_WORLD, &empty_snapshot());
        assert!(!r.passed);
        assert_eq!(r.missing.len(), 2);

        // 缺 description
        let mut s = empty_snapshot();
        s.world_rule_entity_count = 1;
        let r = validate_step(STEP_WORLD, &s);
        assert!(!r.passed);
        assert_eq!(r.missing.len(), 1);
        assert_eq!(r.missing[0].kind, "world_description");

        // 全有
        s.world_description = Some("现代都市".into());
        let r = validate_step(STEP_WORLD, &s);
        assert!(r.passed);
    }

    #[test]
    fn test_flesh_map_step() {
        let r = validate_step(STEP_WORLD_MAP, &empty_snapshot());
        assert!(!r.passed);
        assert!(r.missing.iter().any(|m| m.kind == "location_entity"));

        let mut s = empty_snapshot();
        s.location_entity_count = 1;
        let r = validate_step(STEP_WORLD_MAP, &s);
        assert!(r.passed);
    }

    #[test]
    fn test_flesh_factions_step() {
        let r = validate_step(STEP_WORLD_FACTIONS, &empty_snapshot());
        assert!(!r.passed);
        assert!(r.missing.iter().any(|m| m.kind == "faction_entity"));

        let mut s = empty_snapshot();
        s.faction_entity_count = 1;
        let r = validate_step(STEP_WORLD_FACTIONS, &s);
        assert!(r.passed);
    }

    #[test]
    fn test_flesh_items_step() {
        let r = validate_step(STEP_WORLD_ITEMS, &empty_snapshot());
        assert!(!r.passed);
        assert!(r.missing.iter().any(|m| m.kind == "item_entity"));

        let mut s = empty_snapshot();
        s.item_entity_count = 1;
        let r = validate_step(STEP_WORLD_ITEMS, &s);
        assert!(r.passed);
    }

    #[test]
    fn test_golden_finger_step_requires_relation() {
        let mut s = empty_snapshot();
        s.golden_finger_entity_count = 1;
        let r = validate_step(STEP_GOLDEN_FINGER, &s);
        assert!(!r.passed);
        assert!(r.missing.iter().any(|m| m.kind == "golden_finger_relation"));

        s.golden_finger_has_relation_to_protagonist = true;
        let r = validate_step(STEP_GOLDEN_FINGER, &s);
        assert!(r.passed);
    }

    #[test]
    fn test_protagonist_step() {
        let r = validate_step(STEP_PROTAGONIST, &empty_snapshot());
        assert!(!r.passed);
        assert!(r.missing.iter().any(|m| m.kind == "protagonist"));

        let mut s = empty_snapshot();
        s.character_entity_count = 1;
        let r = validate_step(STEP_PROTAGONIST, &s);
        assert!(r.passed);
    }

    #[test]
    fn test_supporting_step_requires_one_extra() {
        let r = validate_step(STEP_SUPPORTING, &empty_snapshot());
        assert!(!r.passed);
        assert!(r.missing.iter().any(|m| m.kind == "supporting_character"));

        // 1 个主角但没配角：仍不通过
        let mut s = empty_snapshot();
        s.character_entity_count = 1;
        let r = validate_step(STEP_SUPPORTING, &s);
        assert!(!r.passed);

        // 主角 + 1 配角：通过
        s.character_entity_count = 2;
        let r = validate_step(STEP_SUPPORTING, &s);
        assert!(r.passed);
    }

    #[test]
    fn test_storylines_step_requires_all_flesh() {
        // 没有血肉：4 个 flesh missing + main_storyline = 5
        let r = validate_step(STEP_STORYLINES, &empty_snapshot());
        assert!(!r.passed);
        let kinds: Vec<_> = r.missing.iter().map(|m| m.kind.as_str()).collect();
        assert!(kinds.contains(&"flesh_map"));
        assert!(kinds.contains(&"flesh_factions"));
        assert!(kinds.contains(&"flesh_items"));
        assert!(kinds.contains(&"flesh_supporting"));
        assert!(kinds.contains(&"main_storyline"));

        // 全血肉 + 主角：只缺主线
        let mut s = empty_snapshot();
        s.character_entity_count = 2; // 1 主角 + 1 配角
        s.location_entity_count = 1;
        s.faction_entity_count = 1;
        s.item_entity_count = 1;
        let r = validate_step(STEP_STORYLINES, &s);
        assert!(!r.passed);
        assert_eq!(
            r.missing.len(),
            1,
            "应该只缺 main_storyline，实际缺：{:?}",
            r.missing.iter().map(|m| m.kind.as_str()).collect::<Vec<_>>()
        );
        assert_eq!(r.missing[0].kind, "main_storyline");

        // 全有：通过
        s.main_storyline_count = 1;
        s.storyline_total_count = 1;
        let r = validate_step(STEP_STORYLINES, &s);
        assert!(r.passed);
    }

    #[test]
    fn test_storylines_step_requires_main_line_and_all_flesh() {
        let mut s = empty_snapshot();
        s.storyline_total_count = 2;
        // 没有血肉 → 缺 main + 4 个 flesh
        let r = validate_step(STEP_STORYLINES, &s);
        assert!(!r.passed);
        assert!(r.missing.iter().any(|m| m.kind == "main_storyline"));

        // 加血肉
        s.character_entity_count = 2; // 主角+配角
        s.location_entity_count = 1;
        s.faction_entity_count = 1;
        s.item_entity_count = 1;
        // 还缺主线
        let r = validate_step(STEP_STORYLINES, &s);
        assert!(!r.passed);
        assert_eq!(r.missing.len(), 1);
        assert_eq!(r.missing[0].kind, "main_storyline");

        // 全有
        s.main_storyline_count = 1;
        let r = validate_step(STEP_STORYLINES, &s);
        assert!(r.passed);
    }

    #[test]
    fn test_beats_chapters_step_requires_real_chapters() {
        // 空快照必须卡住
        let r = validate_step(STEP_BEATS_CHAPTERS, &empty_snapshot());
        assert!(!r.passed);
        let kinds: Vec<&str> = r.missing.iter().map(|m| m.kind.as_str()).collect();
        assert!(kinds.contains(&"volume_node"), "{:?}", kinds);

        // 有卷、但弧下面一个章都没有 → 卡在空壳弧
        let mut s = empty_snapshot();
        s.volume_node_count = 1;
        s.arcs_without_chapter = 5;
        let r = validate_step(STEP_BEATS_CHAPTERS, &s);
        assert!(!r.passed);
        assert!(r.missing.iter().any(|m| m.kind == "arc_without_chapter"));

        // 章都有了、但全是空壳（没有一句话事件）→ 仍然卡住。
        // 这一条正是旧 `Beats` 判定放过去的情形：AI 会宣布"细纲已就绪"而用户认为没开始。
        s.arcs_without_chapter = 0;
        s.chapter_node_count = 12;
        s.chapters_without_description = 12;
        let r = validate_step(STEP_BEATS_CHAPTERS, &s);
        assert!(!r.passed);
        assert!(r.missing.iter().any(|m| m.kind == "chapter_without_description"));

        // 重要线还没挂节点 → 仍然卡住
        s.chapters_without_description = 0;
        s.important_storylines_without_node = 2;
        let r = validate_step(STEP_BEATS_CHAPTERS, &s);
        assert!(!r.passed);
        assert_eq!(r.missing.len(), 1);
        assert_eq!(r.missing[0].kind, "storyline_without_node");

        // 全齐 → 通过，且下一步是场景步（不再是终点）
        s.important_storylines_without_node = 0;
        let r = validate_step(STEP_BEATS_CHAPTERS, &s);
        assert!(r.passed);
        assert_eq!(r.next_step.as_deref(), Some(STEP_BEATS_SCENES));
    }

    #[test]
    fn test_beats_scenes_step_requires_scene_fields() {
        let mut s = empty_snapshot();
        s.volume_node_count = 1;

        // 一个场景都没有 → 卡住
        let r = validate_step(STEP_BEATS_SCENES, &s);
        assert!(!r.passed);
        assert!(r.missing.iter().any(|m| m.kind == "scene_node"));

        // 场景够数但必填字段不全 → 卡住
        s.scene_node_count = 3;
        s.scenes_missing_required_fields = 2;
        let r = validate_step(STEP_BEATS_SCENES, &s);
        assert!(!r.passed);
        assert!(r.missing.iter().any(|m| m.kind == "scene_missing_fields"));

        // 字段齐 → 通过，且下一步是正文
        s.scenes_missing_required_fields = 0;
        let r = validate_step(STEP_BEATS_SCENES, &s);
        assert!(r.passed);
        assert_eq!(r.next_step.as_deref(), Some(STEP_WRITING));
    }

    #[test]
    fn test_writing_is_terminal_step() {
        // 终点站：进入即通过，没有下一步
        let r = validate_step(STEP_WRITING, &empty_snapshot());
        assert!(r.passed);
        assert_eq!(r.next_step, None);
        assert_eq!(next_step_key(STEP_WRITING), None);
    }

    #[test]
    fn test_legacy_beats_key_gives_migration_hint() {
        // 拆分前的 `beats` 必须给出可执行的迁移提示，而不是笼统的「未知阶段」
        let r = validate_step(LEGACY_STEP_BEATS, &empty_snapshot());
        assert!(!r.passed);
        assert_eq!(r.missing.len(), 1);
        assert_eq!(r.missing[0].kind, "legacy_step_key");
        assert!(
            r.missing[0].detail.contains(STEP_BEATS_CHAPTERS),
            "迁移提示里应写明新 key：{}",
            r.missing[0].detail
        );
    }

    #[test]
    fn test_validation_report_serializes_as_json() {
        // 验证结构化报告能直接 JSON 序列化（agent 端要直接读这个 JSON）
        let r = validate_step(STEP_PREMISE, &empty_snapshot());
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("\"passed\":false"));
        assert!(json.contains("\"kind\":\"project_premise\""));
    }
}
