import { api } from './client'
import type { Entity } from '@/types'

export const locationApi = {
  list: (worldId: string) => api.get<Entity[]>(`/worlds/${worldId}/locations`),
  get: (id: string) => api.get<Entity>(`/locations/${id}`),
  create: (worldId: string, data: Partial<Entity>) => api.post<Entity>(`/worlds/${worldId}/locations`, data),
  update: (id: string, data: Partial<Entity>) => api.put<Entity>(`/locations/${id}`, data),
  delete: (id: string) => api.delete<void>(`/locations/${id}`),
  getEntities: (id: string) => api.get<Entity[]>(`/locations/${id}/entities`),
  getEvents: (id: string) => api.get<any[]>(`/locations/${id}/events`),
}
