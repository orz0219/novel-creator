// Project types

import type { Timestamps } from './common'

export type ProjectStatus =
  | 'Concept'
  | 'Planning'
  | 'Writing'
  | 'Paused'
  | 'Completed'
  | 'Archived'

export interface Project extends Timestamps {
  id: string
  name: string
  description?: string
  language?: string
  world_setting?: string
  system_setting?: string
  default_model?: string
  default_style?: string
  default_params: Record<string, unknown>
  config: Record<string, unknown>
  status: ProjectStatus
}

export interface CreateProjectInput {
  name: string
  description?: string
  language?: string
  world_setting?: string
}

export interface UpdateProjectInput {
  name?: string
  description?: string
  status?: ProjectStatus
  default_model?: string
  default_style?: string
  config?: Record<string, unknown>
}
