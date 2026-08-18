// Character types

import type { Timestamps } from './common'

export interface CharacterProfile extends Timestamps {
  id: string
  entity_id: string
  real_name?: string
  nickname?: string
  age?: string
  gender?: string
  identity?: string
  appearance?: string
  background?: string
  social_status?: string
  core_personality?: string
  values?: string
}

export interface CharacterState extends Timestamps {
  id: string
  entity_id: string
  location?: string
  health?: string
  cultivation?: string
  resources?: string
  current_status?: string
  emotion?: string
  short_term_goal?: string
  long_term_goal?: string
  immediate_intention?: string
}
