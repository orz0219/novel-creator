// AI 可追溯 API（generation_run / validation_run 只读视图）
import { api } from './client'

export interface GenerationRun {
  id: string
  task_id: string
  context_snapshot_id: string | null
  llm_model: string
  provider: string | null
  prompt_sent: string
  response_received: string
  token_usage: Record<string, unknown> | null
  latency_ms: number | null
  reproducibility_meta: Record<string, unknown> | null
  created_at: string
}

export interface ValidationIssue {
  id: string
  issue_type: string
  severity: string
  message: string
  suggestion: string | null
}

export interface ValidationRun {
  id: string
  task_id: string
  changes_validated: number
  changes_approved: number
  changes_rejected: number
  status: string
  started_at: string
  completed_at: string | null
  issues: ValidationIssue[]
}

export const traceApi = {
  listGenerationRuns: (projectId: string, limit = 50) =>
    api.get<GenerationRun[]>(`/projects/${projectId}/generation-runs?limit=${limit}`),
  listValidationRuns: (projectId: string, limit = 50) =>
    api.get<ValidationRun[]>(`/projects/${projectId}/validation-runs?limit=${limit}`),
}
