// Narrative structure types

import type { Timestamps } from './common'

export type NarrativeNodeType =
  | 'Volume'
  | 'Arc'
  | 'Sequence'
  | 'Chapter'
  | 'Scene'
  | 'Beat'
  | 'Storyline'
  | 'SubArc'
  | 'Special'
  // 自定义类型必须显式带 custom: 前缀（后端严格校验，拼错会报错而不是静默降级）
  | `custom:${string}`

export type NarrativeNodeStatus =
  | 'Draft'
  | 'Planned'
  | 'InProgress'
  | 'Completed'
  | 'Archived'

/** 一条「节点服务哪条故事线的哪个阶段」的引用（附加挂载，可多条） */
export interface NarrativeStageRef {
  storyline_id: string
  arc_stage?: string
}

export interface NarrativeNode extends Timestamps {
  id: string
  project_id: string
  world_id: string
  node_type: NarrativeNodeType
  parent_id?: string
  title: string
  description?: string
  content?: string
  attributes: Record<string, unknown>
  sort_order: number
  status: NarrativeNodeStatus
  /** 主挂载：这条节点服务的故事线 */
  storyline_id?: string
  /** 故事线名字（后端 join 带出，省一次往返） */
  storyline_name?: string
  /** 主挂载：推进到该故事线的哪个阶段 */
  arc_stage?: string
  /** 附加挂载：多条线 × 多个阶段 */
  stage_refs?: NarrativeStageRef[]
  /** 场景级挂载：在场角色 */
  participant_entity_ids?: string[]
  /** 场景级挂载：地点 */
  location_id?: string
  /** 场景级挂载：道具 */
  item_ids?: string[]
  /** 预计章数 */
  estimated_chapters?: number
  /** 预计字数 */
  estimated_words?: number
  /** 故事内时间跨度（自由文本） */
  story_time?: string
  /** 直接子节点数（后端带出，用于树上展开） */
  child_count?: number
}

export interface VolumeAttributes {
  mission?: string
  theme?: string
  conflict?: string
  goal?: string
  start_state?: string
  end_state?: string
  important_character_ids: string[]
  important_location_ids: string[]
  major_events: string[]
  secrets: string[]
  foreshadowing: string[]
  resolution?: string
  story_contract_id?: string
}

export interface ArcAttributes {
  goal?: string
  conflict?: string
  participants: string[]
  start_condition?: string
  end_condition?: string
  key_events: string[]
  twists: string[]
  story_contract_id?: string
}

export interface SceneAttributes {
  objective?: string
  conflict?: string
  pov_character_id?: string
  location_id?: string
  time?: string
  emotional_goal?: string
  information_goal?: string
  required_events: string[]
  forbidden_events: string[]
  expected_changes: string[]
  required_facts: string[]
  characters_present: string[]
}

export interface BeatAttributes {
  action: string
  emotion?: string
  dialogue_needed: boolean
  word_count_target?: number
}

// ---- Storyline ----
export type StorylineStatus = 'Planned' | 'Active' | 'Resolved' | 'Abandoned'
export type StorylineImportance = 'Main' | 'Important' | 'Normal' | 'Minor'
/** 明/暗线 */
export type StorylineTone = 'light' | 'dark'
/** 可见性（暗线一般 hidden） */
export type StorylineVisibility = 'visible' | 'hidden'

export interface Storyline extends Timestamps {
  id: string
  project_id: string
  name: string
  description?: string
  status: StorylineStatus
  importance: StorylineImportance
  tone: StorylineTone
  visibility: StorylineVisibility
  created_volume_id?: string
  resolved_volume_id?: string
}

/** 副线挂载关系：child 挂在 parent 下 */
export interface StorylineRelation {
  id: string
  project_id: string
  parent_id: string
  child_id: string
  created_at: string
}

// ---- Foreshadowing ----
export type ForeshadowingStatus = 'Planned' | 'Introduced' | 'Active' | 'Revealed' | 'Abandoned'
export type ForeshadowingImportance = 'Core' | 'Important' | 'Normal' | 'Minor'
export type HintLevel = 'Explicit' | 'Direct' | 'Subtle' | 'Hidden'

export interface Foreshadowing extends Timestamps {
  id: string
  project_id: string
  storyline_id?: string
  name: string
  description?: string
  status: ForeshadowingStatus
  importance: ForeshadowingImportance
  hint_level: HintLevel
  introduced_at?: string
  expected_reveal_at?: string
  actual_reveal_at?: string
}

// Tree node with children (for computed tree)
export interface TreeNode extends NarrativeNode {
  children: TreeNode[]
}
