// Approval types

export type ApprovalStatus = 'Pending' | 'Approved' | 'Rejected' | 'NeedsEdit' | 'Expired'

export type ApprovalTargetType =
  | 'World'
  | 'Entity'
  | 'Volume'
  | 'Arc'
  | 'Scene'
  | 'Storyline'
  | 'Fact'
  | { Custom: string }

export interface ApprovalRecord {
  id: string
  project_id: string
  target_type: ApprovalTargetType
  target_id: string
  status: ApprovalStatus
  reviewer?: string
  review_notes?: string
  created_at: string
  reviewed_at?: string
}
