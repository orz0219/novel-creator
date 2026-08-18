// Generation types

export type GenerationTaskType =
  | 'GenerateScene'
  | 'RewriteSelection'
  | 'ExpandParagraph'
  | 'GenerateLocation'
  | 'GenerateCharacter'
  | 'AnalyzeCharacter'
  | 'CheckConsistency'
  | 'GenerateArc'
  | 'Custom'

export type GenerationTaskStatus =
  | 'Pending'
  | 'BuildingContext'
  | 'Generating'
  | 'Validating'
  | 'Completed'
  | 'Failed'
  | 'Cancelled'

export interface GenerationTask {
  id: string
  type: GenerationTaskType
  target_id?: string
  model?: string
  parameters: Record<string, unknown>
  status: GenerationTaskStatus
  context_tokens?: number
  result?: string
  error?: string
  created_at: string
  updated_at: string
}

export interface GenerationProgressEvent {
  task_id: string
  status: GenerationTaskStatus
  message?: string
  progress?: number
}
