// Proposal types

import type { StateChange } from './world'

export type ProposalStatus = 'Pending' | 'Approved' | 'Rejected' | 'PartiallyAccepted' | 'Expired'

export interface Proposal {
  id: string
  generation_task_id: string
  status: ProposalStatus
  changes: ProposalChange[]
  validation_results: ValidationResult[]
  reason?: string
  created_at: string
  reviewed_at?: string
}

export interface ProposalChange {
  id: string
  change_type: 'Added' | 'Removed' | 'Modified'
  target_entity_type: string
  target_entity_id?: string
  target_entity_name: string
  state_change?: StateChange
  description: string
  risk_level: 'Low' | 'Medium' | 'High'
  accepted?: boolean
}

export type ValidationSeverity = 'Error' | 'Warning' | 'Info'

export interface ValidationResult {
  id: string
  severity: ValidationSeverity
  dimension: string
  message: string
  suggestion?: string
  related_entity_ids: string[]
}
