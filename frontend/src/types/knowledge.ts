// Knowledge types

import type { Timestamps } from './common'

export type KnowledgeSubjectType = 'Author' | 'Character' | 'Reader' | 'Faction'
export type KnowledgeLevel = 'Unknown' | 'Hearsay' | 'Partial' | 'Complete' | 'Misunderstood' | 'FalseBelief'

export interface KnowledgeState extends Timestamps {
  id: string
  project_id: string
  fact_id: string
  subject_type: KnowledgeSubjectType
  subject_id?: string
  knows: boolean
  knowledge_level: KnowledgeLevel
  source?: string
  effective_from: string
  effective_to?: string
}

export type ReaderKnowledgeLevel = 'Unknown' | 'Hearsay' | 'Suspected' | 'Partial' | 'Complete' | 'Misunderstood'
export type ReaderConfidence = 'Certain' | 'Likely' | 'Uncertain' | 'Speculative'

export interface ReaderKnowledge extends Timestamps {
  id: string
  project_id: string
  fact_id: string
  knowledge_level: ReaderKnowledgeLevel
  source_scene_id?: string
  confidence: ReaderConfidence
}
