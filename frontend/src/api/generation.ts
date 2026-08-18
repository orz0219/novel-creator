// Generation API
import { api, createSSE } from './client'
import type { GenerationTask } from '@/types'

export const generationApi = {
  list: (projectId: string) => api.get<GenerationTask[]>(`/projects/${projectId}/generations`),
  get: (id: string) => api.get<GenerationTask>(`/generations/${id}`),
  start: (projectId: string, data: { type: string; target_id?: string; model?: string; parameters?: Record<string, unknown> }) =>
    api.post<GenerationTask>(`/projects/${projectId}/generations`, data),
  cancel: (id: string) => api.post<void>(`/generations/${id}/cancel`),
  stream: (taskId: string, onMessage: (event: MessageEvent) => void) =>
    createSSE(`/generations/${taskId}/stream`, onMessage),
}
