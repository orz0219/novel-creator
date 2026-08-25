// Rules API
import { api } from './client'

// 后端真源 crates/domain/src/canon.rs:79-96（CanonRule）。
// rule_level 序列化值见 RuleLevel::as_str()：'RULE-0' | 'RULE-1' | 'RULE-2' | 'RULE-3'
// enforcement 序列化值见 EnforcementAction::as_str()：'Reject' | 'RequireApproval' | 'Allow'
export interface CanonRule {
  id: string
  project_id: string
  world_id: string
  rule_level: string
  rule_content: string
  affected_scope: string
  enforcement: string
  constraints?: Record<string, unknown> | null
  source?: string | null
  created_at: string
  updated_at: string
}

export const rulesApi = {
  list: (worldId: string) => api.get<CanonRule[]>(`/worlds/${worldId}/rules`),
  get: (id: string) => api.get<CanonRule>(`/rules/${id}`),
  create: (worldId: string, data: { rule_content: string; rule_level?: string; affected_scope?: string; enforcement?: string }) =>
    api.post<CanonRule>(`/worlds/${worldId}/rules`, data),
  update: (id: string, data: Partial<CanonRule>) => api.put<CanonRule>(`/rules/${id}`, data),
  delete: (id: string) => api.delete<void>(`/rules/${id}`),
}
