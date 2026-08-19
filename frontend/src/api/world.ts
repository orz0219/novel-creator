// World API
import { api } from './client'
import type { World, Entity, Relation, Event, Fact } from '@/types'

export const worldApi = {
  get: (projectId: string) => api.get<World>(`/projects/${projectId}/world`),
  update: (projectId: string, data: Partial<World>) => api.put<World>(`/projects/${projectId}/world`, data),
  listEntities: (worldId: string, type?: string) => api.get<Entity[]>(`/worlds/${worldId}/entities${type ? '?type=' + type : ''}`),
}

export const entityApi = {
  list: (worldId: string, type?: string) => api.get<Entity[]>(`/worlds/${worldId}/entities${type ? '?type=' + type : ''}`),
  get: (id: string) => api.get<Entity>(`/entities/${id}`),
  create: (worldId: string, data: Partial<Entity>) => api.post<Entity>(`/worlds/${worldId}/entities`, data),
  createCharacter: (worldId: string, data: Partial<Entity>) => api.post<Entity>(`/worlds/${worldId}/characters`, data),
  createLocation: (worldId: string, data: Partial<Entity>) => api.post<Entity>(`/worlds/${worldId}/locations`, data),
  createFaction: (worldId: string, data: Partial<Entity>) => api.post<Entity>(`/worlds/${worldId}/factions`, data),
  update: (id: string, data: Partial<Entity>) => api.put<Entity>(`/entities/${id}`, data),
  delete: (id: string) => api.delete<void>(`/entities/${id}`),
}

export const relationApi = {
  list: (worldId: string) => api.get<Relation[]>(`/worlds/${worldId}/relations`),
  create: (worldId: string, data: Partial<Relation>) => api.post<Relation>(`/worlds/${worldId}/relations`, data),
  delete: (id: string) => api.delete<void>(`/relations/${id}`),
}

export const eventApi = {
  list: (projectId: string) => api.get<Event[]>(`/projects/${projectId}/events`),
  create: (projectId: string, data: Partial<Event>) => api.post<Event>(`/projects/${projectId}/events`, data),
}

export const factApi = {
  list: (projectId: string) => api.get<Fact[]>(`/projects/${projectId}/facts`),
  create: (projectId: string, data: Partial<Fact>) => api.post<Fact>(`/projects/${projectId}/facts`, data),
}
