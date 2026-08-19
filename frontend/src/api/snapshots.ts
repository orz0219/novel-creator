// Snapshots API
import { api } from './client'

export interface Snapshot {
  id: string
  name: string
  story_time: string
  world_summary: string
  current_location: string
  active_threads_count: number
  unresolved_foreshadows_count: number
  known_characters_count: number
  known_locations_count: number
  progress: string
  created_at: string
}

export const snapshotsApi = {
  list: (projectId: string) => api.get<Snapshot[]>(`/projects/${projectId}/snapshots`),
  create: (projectId: string, data: { name?: string; story_time?: string; world_summary?: string }) =>
    api.post<Snapshot>(`/projects/${projectId}/snapshots`, data),
  delete: (id: string) => api.delete<void>(`/snapshots/${id}`),
}
