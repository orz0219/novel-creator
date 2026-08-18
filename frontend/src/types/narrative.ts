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

export type NarrativeNodeStatus =
  | 'Draft'
  | 'Planned'
  | 'InProgress'
  | 'Completed'
  | 'Archived'

export interface NarrativeNode extends Timestamps {
  id: string
  project_id: string
  world_id: string
  node_type: NarrativeNodeType
  parent_id?: string
  title: string
  description?: string
  attributes: Record<string, unknown>
  sort_order: number
  status: NarrativeNodeStatus
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

export interface Storyline extends Timestamps {
  id: string
  project_id: string
  name: string
  description?: string
  status: StorylineStatus
  importance: StorylineImportance
  created_volume_id?: string
  resolved_volume_id?: string
}

// ---- Foreshadowing ----
export type ForeshadowingStatus = 'Planned' | 'Introduced' | 'Active' | 'Revealed' | 'Abandoned'
export type ForeshadowingImportance = 'Core' | 'Important' | 'Normal' | 'Minor'
export type HintLevel = 'Explicit' | 'Direct' | 'Subtle' | 'Hidden'

export interface Foreshadowing extends Timestamps {
  id: string
  project_id: string
  name: string
  description?: string
  status: ForeshadowingStatus
  importance: ForeshadowingImportance
  hint_level: HintLevel
  planted_scene_id?: string
  revealed_scene_id?: string
  related_entity_ids: string[]
}

// Tree node with children (for computed tree)
export interface TreeNode extends NarrativeNode {
  children: TreeNode[]
}
