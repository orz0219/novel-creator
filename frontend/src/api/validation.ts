import { api } from './client'
import type { ValidationResult } from '@/types'

export const validationApi = {
  validateScene: (sceneId: string) => api.post<ValidationResult[]>(`/scenes/${sceneId}/validate`),
  validateProposal: (proposalId: string) => api.post<ValidationResult[]>(`/proposals/${proposalId}/validate`),
  validateWorld: (worldId: string) => api.post<ValidationResult[]>(`/worlds/${worldId}/validate`),
}
