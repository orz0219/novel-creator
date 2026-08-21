// Story / Narrative API
import { api } from './client'
import type { NarrativeNode, Storyline, Foreshadowing } from '@/types'

export const narrativeApi = {
  listNodes: (projectId: string) => api.get<NarrativeNode[]>(`/projects/${projectId}/narrative`),
  getNode: (id: string) => api.get<NarrativeNode>(`/narrative/${id}`),
  createNode: (projectId: string, data: Partial<NarrativeNode>) => api.post<NarrativeNode>(`/projects/${projectId}/narrative`, data),
  updateNode: (id: string, data: Partial<NarrativeNode>) => api.put<NarrativeNode>(`/narrative/${id}`, data),
  deleteNode: (id: string) => api.delete<void>(`/narrative/${id}`),
}

export const storylineApi = {
  list: (projectId: string) => api.get<Storyline[]>(`/projects/${projectId}/storylines`),
  create: (projectId: string, data: Partial<Storyline>) => api.post<Storyline>(`/projects/${projectId}/storylines`, data),
  update: (id: string, data: Partial<Storyline>) => api.put<Storyline>(`/storylines/${id}`, data),
  delete: (id: string) => api.delete<void>(`/storylines/${id}`),
}

export const foreshadowApi = {
  list: (projectId: string) => api.get<Foreshadowing[]>(`/projects/${projectId}/foreshadows`),
  create: (projectId: string, data: Partial<Foreshadowing>) => api.post<Foreshadowing>(`/projects/${projectId}/foreshadows`, data),
  update: (id: string, data: Partial<Foreshadowing>) => api.put<Foreshadowing>(`/foreshadows/${id}`, data),
  delete: (id: string) => api.delete<void>(`/foreshadows/${id}`),
}
