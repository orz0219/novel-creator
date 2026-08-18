// Proposal API
import { api } from './client'
import type { Proposal } from '@/types'

export const proposalApi = {
  list: (projectId: string) => api.get<Proposal[]>(`/projects/${projectId}/proposals`),
  get: (id: string) => api.get<Proposal>(`/proposals/${id}`),
  accept: (id: string) => api.post<void>(`/proposals/${id}/accept`),
  reject: (id: string) => api.post<void>(`/proposals/${id}/reject`),
  acceptChange: (proposalId: string, changeId: string) =>
    api.post<void>(`/proposals/${proposalId}/changes/${changeId}/accept`),
  rejectChange: (proposalId: string, changeId: string) =>
    api.post<void>(`/proposals/${proposalId}/changes/${changeId}/reject`),
}
