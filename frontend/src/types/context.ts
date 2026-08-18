// Context Engine types

export type ContextPolicyType = 'Automatic' | 'Pinned' | 'Excluded'

export interface ContextEntity {
  entity_id: string
  entity_name: string
  entity_type: string
  relevance: number
  reasons: string[]
  policy: ContextPolicyType
}

export interface ContextItem {
  id: string
  type: 'entity' | 'relationship' | 'timeline' | 'knowledge' | 'constraint' | 'history'
  content: string
  relevance: number
  source: string
}

export interface ContextSnapshot {
  id: string
  scene_id?: string
  entities: ContextEntity[]
  items: ContextItem[]
  total_tokens: number
  created_at: string
}
