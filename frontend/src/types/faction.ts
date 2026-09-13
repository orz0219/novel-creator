// Faction types

import type { Timestamps } from './common'
import type { EntityArcStage } from './character'

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
  /** 阶段弧线：势力在不同阶段的目标 / 戏份 / 实力快照（可选，不填则不显示） */
  arc_stages?: EntityArcStage[]
}
