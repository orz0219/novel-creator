import { api } from './client'
import type { Entity } from '@/types'
import type { CharacterProfile, CharacterState } from '@/types'

export const characterApi = {
  list: (worldId: string) => api.get<Entity[]>(`/worlds/${worldId}/characters`),
  get: (id: string) => api.get<Entity>(`/characters/${id}`),
  getProfile: (id: string) => api.get<CharacterProfile>(`/characters/${id}/profile`),
  getState: (id: string) => api.get<CharacterState>(`/characters/${id}/state`),
  create: (worldId: string, data: Partial<Entity>) => api.post<Entity>(`/worlds/${worldId}/characters`, data),
  update: (id: string, data: Partial<Entity>) => api.put<Entity>(`/characters/${id}`, data),
  delete: (id: string) => api.delete<void>(`/characters/${id}`),
  getKnowledge: (id: string) => api.get<any[]>(`/characters/${id}/knowledge`),
  getRelationships: (id: string) => api.get<any[]>(`/characters/${id}/relationships`),
}
