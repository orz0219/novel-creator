import { api } from './client'
import type { Entity } from '@/types'
import type { CharacterProfile, CharacterState, LocationProfile, FactionProfile } from '@/types'

export const characterApi = {
  list: (worldId: string) => api.get<Entity[]>(`/worlds/${worldId}/characters`),
  get: (id: string) => api.get<Entity>(`/characters/${id}`),
  getProfile: (id: string) => api.get<CharacterProfile>(`/characters/${id}/profile`),
  getState: (id: string) => api.get<CharacterState>(`/characters/${id}/state`),
  updateProfile: (id: string, data: Partial<CharacterProfile>) => api.put<CharacterProfile>(`/characters/${id}/profile`, data),
  updateState: (id: string, data: Partial<CharacterState>) => api.put<CharacterState>(`/characters/${id}/state`, data),
  create: (worldId: string, data: Partial<Entity>) => api.post<Entity>(`/worlds/${worldId}/characters`, data),
  update: (id: string, data: Partial<Entity>) => api.put<Entity>(`/characters/${id}`, data),
  delete: (id: string) => api.delete<void>(`/characters/${id}`),
  getKnowledge: (id: string) => api.get<any[]>(`/characters/${id}/knowledge`),
  getRelationships: (id: string) => api.get<any[]>(`/characters/${id}/relationships`),
}

export const locationProfileApi = {
  get: (id: string) => api.get<LocationProfile>(`/locations/${id}/profile`),
  upsert: (id: string, data: Partial<LocationProfile>) => api.put<LocationProfile>(`/locations/${id}/profile`, data),
}

export const factionProfileApi = {
  get: (id: string) => api.get<FactionProfile>(`/factions/${id}/profile`),
  upsert: (id: string, data: Partial<FactionProfile>) => api.put<FactionProfile>(`/factions/${id}/profile`, data),
}
