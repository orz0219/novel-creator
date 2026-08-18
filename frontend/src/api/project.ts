// Project API
import { api } from './client'
import type { Project, CreateProjectInput, UpdateProjectInput } from '@/types'

export const projectApi = {
  list: () => api.get<Project[]>('/projects'),
  get: (id: string) => api.get<Project>(`/projects/${id}`),
  create: (input: CreateProjectInput) => api.post<Project>('/projects', input),
  update: (id: string, input: UpdateProjectInput) => api.put<Project>(`/projects/${id}`, input),
  delete: (id: string) => api.delete<void>(`/projects/${id}`),
}
