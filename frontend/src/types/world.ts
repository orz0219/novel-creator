// World, Entity, Relationship, Fact, Event types

import type { Timestamps } from './common'

// ---- World ----
export interface World extends Timestamps {
  id: string
  project_id: string
  name: string
  description?: string
  world_rules?: string
  config: Record<string, unknown>
  is_main: boolean
}

// ---- Entity ----
export interface Entity extends Timestamps {
  id: string
  project_id: string
  world_id: string
  entity_type_id: string
  name: string
  summary?: string
  description?: string
  attributes: Record<string, unknown>
  version: number
  created_by: string
  updated_by?: string
  source_generation_id?: string
}

export interface EntityType extends Timestamps {
  id: string
  name: string
  description?: string
  schema?: Record<string, unknown>
}

export const ENTITY_TYPE_NAMES = {
  CHARACTER: 'Character',
  LOCATION: 'Location',
  FACTION: 'Faction',
  ITEM: 'Item',
  CREATURE: 'Creature',
  ORGANIZATION: 'Organization',
  NATION: 'Nation',
  CITY: 'City',
  SECT: 'Sect',
  RACE: 'Race',
  DEITY: 'Deity',
  TECHNOLOGY: 'Technology',
  CONCEPT: 'Concept',
} as const

// ---- Relationship ----
export interface Relation extends Timestamps {
  id: string
  project_id: string
  source_entity_id: string
  target_entity_id: string
  relation_type: string
  description?: string
  attributes: Record<string, unknown>
  valid_from?: string
  valid_until?: string
}

// ---- Fact ----
export type FactCertainty = 'Confirmed' | 'Likely' | 'Rumor' | 'Uncertain'

export interface Fact extends Timestamps {
  id: string
  project_id: string
  content: string
  category?: string
  certainty: FactCertainty
}

// ---- Event ----
export interface Event extends Timestamps {
  id: string
  project_id: string
  name: string
  description: string
  event_type?: string
  timestamp?: string
  event_time?: string
  duration?: string
  involved_entity_ids: string[]
  state_changes: StateChange[]
}

export interface StateChange {
  change_type: StateChangeType
  target_entity_id: string
  state_key: string
  old_value?: unknown
  new_value: unknown
}

export type StateChangeType =
  | 'LocationChange'
  | 'StatusChange'
  | 'AttributeChange'
  | 'RelationshipChange'
  | 'ResourceChange'
  | 'KnowledgeChange'
  | { Custom: string }
