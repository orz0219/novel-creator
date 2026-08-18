// Context API
import { api } from './client'
import type { ContextSnapshot } from '@/types'

export const contextApi = {
  getSceneContext: (sceneId: string) => api.get<ContextSnapshot>(`/scenes/${sceneId}/context`),
  buildContext: (sceneId: string) => api.post<ContextSnapshot>(`/scenes/${sceneId}/context/build`),
  pinEntity: (sceneId: string, entityId: string) => api.post<void>(`/scenes/${sceneId}/context/pin/${entityId}`),
  unpinEntity: (sceneId: string, entityId: string) => api.delete<void>(`/scenes/${sceneId}/context/pin/${entityId}`),
  excludeEntity: (sceneId: string, entityId: string) => api.post<void>(`/scenes/${sceneId}/context/exclude/${entityId}`),
  unexcludeEntity: (sceneId: string, entityId: string) => api.delete<void>(`/scenes/${sceneId}/context/exclude/${entityId}`),
}
