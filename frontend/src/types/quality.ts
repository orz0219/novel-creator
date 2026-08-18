// Quality Score types

export interface QualityScore {
  id: string
  project_id: string
  scene_id: string
  run_id?: string
  continuity_score?: number
  character_score?: number
  plot_score?: number
  knowledge_score?: number
  world_score?: number
  style_score?: number
  overall_score?: number
  issues: QualityIssue[]
  created_at: string
}

export interface QualityIssue {
  dimension: string
  severity: string
  description: string
  suggestion?: string
}
