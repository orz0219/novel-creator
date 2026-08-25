// Snapshots API
import { api } from './client'

// 后端真源 crates/domain/src/novel_snapshot.rs:18-46（NovelStateSnapshot）。
// 注意：后端无 name / progress 字段，故已删除。
export interface Snapshot {
  id: string
  project_id: string
  scene_id?: string | null
  story_time?: string | null
  world_summary?: string | null
  main_character_state?: string | null
  current_location?: string | null
  active_threads_count: number
  unresolved_foreshadows_count: number
  known_characters_count: number
  known_locations_count: number
  current_volume_id?: string | null
  current_arc_id?: string | null
  state_data: Record<string, unknown>
  created_at: string
}

export interface RestoreResult {
  restored: boolean
  snapshot_id: string
  project_id: string
  restored_keys: string[]
}

export const snapshotsApi = {
  list: (projectId: string) => api.get<Snapshot[]>(`/projects/${projectId}/snapshots`),
  create: (
    projectId: string,
    data: {
      scene_id?: string
      story_time?: string
      world_summary?: string
      main_character_state?: string
      current_location?: string
      current_volume_id?: string
      current_arc_id?: string
      state_data?: Record<string, unknown>
    },
  ) => api.post<Snapshot>(`/projects/${projectId}/snapshots`, data),
  delete: (id: string) => api.delete<void>(`/snapshots/${id}`),
  restore: (id: string) => api.post<RestoreResult>(`/snapshots/${id}/restore`),
}
