// Faction types

import type { Timestamps } from './common'

export interface FactionProfile extends Timestamps {
  id: string
  entity_id: string
  goals?: string
  leader?: string
  values?: string
  resources?: string
  territory?: string
  members?: string
  enemies?: string
  allies?: string
  internal_conflicts?: string
  secrets?: string
  modus_operandi?: string
}
