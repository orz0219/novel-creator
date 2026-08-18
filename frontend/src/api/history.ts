import { api } from './client'

export interface EventLogEntry {
  id: string
  event_type: string
  entity_id?: string
  entity_type?: string
  description: string
  actor: string
  created_at: string
}

export interface VersionEntry {
  id: string
  entity_id: string
  version: number
  description: string
  actor: string
  created_at: string
  changes: Record<string, { old: unknown; new: unknown }>
}

export const historyApi = {
  getEvents: (projectId: string, limit?: number) =>
    api.get<EventLogEntry[]>(`/projects/${projectId}/events${limit ? '?limit=' + limit : ''}`),
  getVersions: (entityId: string) =>
    api.get<VersionEntry[]>(`/entities/${entityId}/versions`),
  getVersion: (entityId: string, version: number) =>
    api.get<VersionEntry>(`/entities/${entityId}/versions/${version}`),
  compareVersions: (entityId: string, from: number, to: number) =>
    api.get<Record<string, { old: unknown; new: unknown }>>(`/entities/${entityId}/versions/compare?from=${from}&to=${to}`),
}
