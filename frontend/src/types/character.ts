// Character types

import type { Timestamps } from './common'

// 后端 AgeRange（crates/domain/src/character.rs:17）为枚举字符串，不是 {min,max}
export type AgeRange =
  | 'Child'
  | 'Teen'
  | 'YoungAdult'
  | 'Adult'
  | 'MiddleAge'
  | 'Elder'
  | 'Unknown'

export type CharacterGender = 'Male' | 'Female' | 'NonBinary' | 'Unknown' | 'Other'

export type StoryRole =
  | 'Protagonist'
  | 'Antagonist'
  | 'Mentor'
  | 'Ally'
  | 'Rival'
  | 'Catalyst'
  | 'Victim'
  | 'Observer'

/**
 * 后端 social_position 是**结构体**（crates/domain/src/character.rs:156），不是字符串。
 * 此前这里声明成 string，于是页面上「社会地位」用文本输入框，
 * 提交的字符串在后端反序列化失败并被静默丢弃——这正是它一直存不进去的原因。
 */
export interface SocialPosition {
  /** 显式传 null 表示清空该字段（后端 Option<String> 接受 null） */
  rank?: string | null
  authority_level?: number | null
  social_access?: string[]
}

/** 后端 narrative_necessity 是结构体（crates/domain/src/character.rs:164） */
export interface NarrativeNecessity {
  importance?: number
  irreplaceability?: number
  absence_effect?: string
  replacement_cost?: string
}

/** 戏份权重：某一故事阶段实体的出场分量（后端 ScreenWeight 枚举） */
export type ScreenWeight = 'Light' | 'Medium' | 'Heavy'

/**
 * 阶段弧线（外部时间线）：实体在故事不同阶段的身份 / 戏份 / 目标 / 现状。
 *
 * 人物 / 势力 / 地点共用同一结构。
 * 与 `arc_potential`（内在曲线）正交——后者管"为什么会变"，
 * 这个管"何时上场、演什么、戏份多大"。0 条表示不填，不影响现有数据。
 */
export interface EntityArcStage {
  id?: string
  /** 阶段名：前期 / 中期 / 后期，或卷1 / 卷2 */
  stage: string
  /** 排序，越小越早 */
  order?: number
  /** 此阶段的身份 / 功能位 */
  role?: string | null
  screen_weight?: ScreenWeight | null
  goal?: string | null
  /** 此阶段的叙事功能 */
  function?: string | null
  /** 什么事件把它推进这一阶段 */
  entry_trigger?: string | null
  /** 该阶段的现状快照（势力：多少人/占哪/盟友是谁） */
  status?: string | null
}

/** 冲突（后端 CharacterConflict），phase = 该冲突从哪个阶段开始成立 */
export interface CharacterConflict {
  id?: string
  conflict_type?: string
  description: string
  resolution_status?: string | null
  phase?: string | null
}

export interface CharacterProfile extends Timestamps {
  id: string
  entity_id: string
  name?: string
  aliases?: string[]
  /** 后端列名是 age_range（枚举），不是自由文本的 age */
  age_range?: AgeRange
  gender?: CharacterGender
  identity?: string
  appearance?: string
  background_origin?: string
  social_position?: SocialPosition
  core_personality?: string
  values?: string
  role_in_story?: StoryRole
  narrative_necessity?: NarrativeNecessity
  /** 阶段弧线：前中后期的身份 / 戏份 / 目标（可选，不填则不显示） */
  arc_stages?: EntityArcStage[]
  conflicts?: CharacterConflict[]
}

export interface CharacterState extends Timestamps {
  id?: string
  entity_id?: string
  location?: string
  physical_state?: string
  mental_state?: string
  resource_state?: string
  social_state?: string
  flags?: string[]
  extra?: unknown
}

export interface LocationProfile {
  geography?: string
  appearance?: string
  population?: string
  economy?: string
  rules?: string
  history?: string
  narrative_usage?: string
  location_type?: string
  size?: string
  climate?: string
  era?: string
  accessibility?: string
  /** 阶段弧线：该地点在不同阶段承担的叙事角色（可选，不填则不显示） */
  arc_stages?: EntityArcStage[]
}
