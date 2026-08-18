// Common types shared across the application

export interface Timestamps {
  created_at: string
  updated_at: string
}

export interface PaginatedResponse<T> {
  items: T[]
  total: number
  page: number
  page_size: number
}

export interface ApiError {
  code: string
  message: string
  details?: Record<string, unknown>
}

export type SortDirection = 'asc' | 'desc'

export interface SortOptions {
  field: string
  direction: SortDirection
}

export interface FilterOptions {
  [key: string]: string | number | boolean | undefined
}
