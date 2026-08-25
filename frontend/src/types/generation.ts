// Generation types

export type GenerationTaskStatus =
  | 'Pending'
  | 'Running'
  | 'Completed'
  | 'Failed'
  | 'Cancelled'

export interface GenerationTask {
  id: string
  project_id: string
  skill_id?: string
  scene_id?: string
  input: Record<string, unknown> | unknown
  output?: unknown
  status: GenerationTaskStatus
  token_usage?: {
    prompt_tokens?: number
    completion_tokens?: number
    total_tokens?: number
    [k: string]: unknown
  }
  error?: string
  created_at: string
  completed_at?: string
}

export interface GenerationProgressEvent {
  task_id: string
  status: GenerationTaskStatus
  message?: string
  progress?: number
}
