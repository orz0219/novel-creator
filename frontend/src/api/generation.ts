// Generation API
import { api, createSSE } from './client'
import type { GenerationTask } from '@/types'

export const generationApi = {
  list: (projectId: string) => api.get<GenerationTask[]>(`/projects/${projectId}/generations`),
  get: (id: string) => api.get<GenerationTask>(`/generations/${id}`),
  start: (projectId: string, data: { type: string; target_id?: string; model?: string; parameters?: Record<string, unknown> }) =>
    api.post<GenerationTask>(`/projects/${projectId}/generations`, data),
  cancel: (id: string) => api.post<void>(`/generations/${id}/cancel`),
  // 触发后端真正执行生成任务（前端此前只创建 Pending 任务却从不调用，导致任务永远卡在 Pending）。
  execute: (id: string) => api.post<GenerationTask>(`/generations/${id}/execute`),
  stream: (taskId: string, onMessage: (event: MessageEvent) => void) =>
    createSSE(`/generations/${taskId}/stream`, onMessage),
}
