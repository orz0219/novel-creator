// Project API
import { api } from './client'
import type { Project, CreateProjectInput, UpdateProjectInput } from '@/types'

export const projectApi = {
  list: () => api.get<Project[]>('/projects'),
  get: (id: string) => api.get<Project>(`/projects/${id}`),
  create: (input: CreateProjectInput) => api.post<Project>('/projects', input),
  update: (id: string, input: UpdateProjectInput) => api.put<Project>(`/projects/${id}`, input),
  delete: (id: string) => api.delete<void>(`/projects/${id}`),

  /**
   * 导出整个项目（含关联数据）为 JSON 文本，供手机单机版导入。
   * 直接走 fetch 取原文，不走 api.get（后者会强制解析为 JSON 并丢掉原始响应）。
   */
  exportToJson: async (id: string): Promise<string> => {
    const resp = await fetch(`/api/v1/projects/${id}/export`)
    if (!resp.ok) throw new Error(`导出失败 HTTP ${resp.status}`)
    return resp.text()
  },
}
