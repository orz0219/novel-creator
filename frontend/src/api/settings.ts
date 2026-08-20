// 全局应用设置 API
import { api } from './client'

export interface AppSettings {
  projectName?: string
  language?: string
  defaultModel?: string
  fontSize?: number
  autoSave?: boolean
  writingStyle?: string
  autoValidate?: boolean
}

export const settingsApi = {
  get: () => api.get<AppSettings>('/settings'),
  update: (data: AppSettings) => api.put<AppSettings>('/settings', data),
}
