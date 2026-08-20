// Proposal API
import { api } from './client'
import type { Proposal, ExtractionResult } from '@/types'

export const proposalApi = {
  list: (projectId: string) => api.get<Proposal[]>(`/projects/${projectId}/proposals`),
  get: (id: string) => api.get<Proposal>(`/proposals/${id}`),
  accept: (id: string) => api.post<void>(`/proposals/${id}/accept`),
  reject: (id: string) => api.post<void>(`/proposals/${id}/reject`),
  acceptChange: (proposalId: string, changeId: string) =>
    api.post<void>(`/proposals/${proposalId}/changes/${changeId}/accept`),
  rejectChange: (proposalId: string, changeId: string) =>
    api.post<void>(`/proposals/${proposalId}/changes/${changeId}/reject`),
  // M1 文本抽取：把正文发给后端 ExtractionExecutor，返回候选实体/关系并创建草案
  extract: (projectId: string, text: string) =>
    api.post<ExtractionResult>(`/projects/${projectId}/extract`, { text }),
}
