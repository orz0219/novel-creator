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
  id?: string
  entity_id?: string
  location?: string
  health?: string
  cultivation?: string
  money?: string
  wanted?: boolean
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
