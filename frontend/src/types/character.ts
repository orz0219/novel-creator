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

// 后端 Gender/SocialPosition/StoryRole/NarrativeNecessity 均为枚举，序列化为字符串
export type CharacterGender = string
export type SocialPosition = string
export type StoryRole = string
export type NarrativeNecessity = string

export interface CharacterProfile extends Timestamps {
  id: string
  entity_id: string
  name?: string
  aliases?: string[]
  age?: AgeRange
  gender?: CharacterGender
  identity?: string
  appearance?: string
  background_origin?: string
  social_position?: SocialPosition
  core_personality?: string
  values?: string
  role_in_story?: StoryRole
  narrative_necessity?: NarrativeNecessity
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
}
