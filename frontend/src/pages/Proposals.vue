<template>
  <div class="proposals-page">
    <div class="page-header">
      <h1 class="page-title">AI 提案</h1>
    </div>

    <div v-if="loading" class="loading-state">
      <span class="loading-icon">⏳</span>
      <span class="loading-text">加载提案中…</span>
    </div>

    <div v-else-if="proposals.length" class="proposal-list">
      <div v-for="proposal in proposals" :key="proposal.id" class="proposal-card">
        <div class="proposal-header">
          <span class="proposal-id">#{{ proposal.id.split('-')[1] }}</span>
          <span class="status-badge" :class="statusClass(proposal.status)">{{ statusLabels[proposal.status] }}</span>
          <span class="proposal-time">{{ formatDate(proposal.created_at) }}</span>
        </div>
        <div class="proposal-reason" v-if="proposal.reason">
          <span class="reason-label">原因：</span>{{ proposal.reason }}
        </div>

        <!-- Payload: changes -->
        <div class="changes-section">
          <div class="section-title">变更内容 ({{ proposal.changes.length }})</div>
          <div v-for="change in proposal.changes" :key="change.id" class="change-item">
            <span class="change-type" :class="change.change_type.toLowerCase()">{{ changeTypeLabels[change.change_type] }}</span>
            <span class="change-target">{{ change.target_entity_type }}: {{ change.target_entity_name }}</span>
            <span class="change-desc">{{ change.description }}</span>
            <span class="change-risk" :class="change.risk_level.toLowerCase()">{{ riskLabels[change.risk_level] }}</span>
            <div class="change-actions" v-if="proposal.status === 'Pending'">
              <button class="accept-btn" :disabled="changeBusy" @click.stop="acceptChange(proposal, change)">✓</button>
              <button class="reject-btn" :disabled="changeBusy" @click.stop="rejectChange(proposal, change)">✗</button>
            </div>
            <div class="change-state" v-if="change.state_change">
              <span class="state-key">{{ change.state_change.state_key }}</span>
              <span class="state-old">{{ change.state_change.old_value ?? '—' }}</span>
              <span class="state-arrow">→</span>
              <span class="state-new">{{ change.state_change.new_value }}</span>
            </div>
          </div>
        </div>

        <!-- Validation -->
        <div class="validation-section">
          <div class="section-title validation-title">
            验证结果
            <button class="run-validate-btn" :disabled="validatingId === proposal.id" @click="runValidation(proposal)">
              {{ validatingId === proposal.id ? '校验中…' : '运行校验' }}
            </button>
          </div>

          <div v-if="proposal.validation_error" class="validation-error">
            {{ proposal.validation_error }}
          </div>

          <div v-else-if="proposal.validation_results && proposal.validation_results.length" class="validation-list">
            <div v-for="vr in proposal.validation_results" :key="vr.id" class="validation-item">
              <span class="vr-severity" :class="vr.severity.toLowerCase()">{{ severityLabels[vr.severity] }}</span>
              <span class="vr-dimension">{{ vr.dimension }}</span>
              <span class="vr-message">{{ vr.message }}</span>
              <span class="vr-suggestion" v-if="vr.suggestion">建议：{{ vr.suggestion }}</span>
            </div>
          </div>
          <div v-else class="validation-empty">尚未运行校验</div>
        </div>

        <div class="proposal-actions" v-if="proposal.status === 'Pending'">
          <button class="accept-all-btn" :disabled="actionBusy" @click="acceptProposal(proposal)">全部接受</button>
          <button class="reject-all-btn" :disabled="actionBusy" @click="rejectProposal(proposal)">全部拒绝</button>
        </div>
        <div class="proposal-footer">
          <span class="review-time" v-if="proposal.reviewed_at">
            审核于 {{ formatDate(proposal.reviewed_at) }}
          </span>
        </div>
      </div>
    </div>

    <div v-else class="empty-state">
      <div class="empty-icon">📋</div>
      <div class="empty-title">暂无 AI 提案</div>
      <div class="empty-desc">
        提案由 AI 在剧情推进中自动生成。可先在「人物 / 地点 / 势力」页丰富世界观，
        或于写作页使用「AI 生成」触发分析与建议。
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { useProjectStore } from '@/stores/project'
import { proposalApi } from '@/api/proposal'
import type { Proposal, ProposalChange, ValidationResult } from '@/types'
import { validationApi } from '@/api/validation'

const route = useRoute()
const projectStore = useProjectStore()

const projectId = (route.params.id as string) || projectStore.currentProject?.id || ''

const proposals = ref<ProposalVM[]>([])
const loading = ref(false)
const actionBusy = ref(false)
const changeBusy = ref(false)
const validatingId = ref<string | null>(null)

// Proposal status with runtime validation_error appended (not part of the API type)
type ProposalVM = Proposal & { validation_error?: string }

onMounted(async () => {
  loading.value = true
  try {
    proposals.value = await proposalApi.list(projectId).catch(() => [])
  } finally {
    loading.value = false
  }
})

const statusLabels: Record<string, string> = {
  Pending: '待审',
  Approved: '已批准',
  Rejected: '已拒绝',
  PartiallyAccepted: '部分接受',
  Expired: '已过期',
}

const changeTypeLabels: Record<string, string> = {
  Added: '新增',
  Removed: '移除',
  Modified: '修改',
}

const riskLabels: Record<string, string> = {
  Low: '低',
  Medium: '中',
  High: '高',
}

const severityLabels: Record<string, string> = {
  Error: '错误',
  Warning: '警告',
  Info: '信息',
}

function statusClass(status: string): string {
  return `status-${status.toLowerCase()}`
}

async function acceptProposal(proposal: Proposal) {
  actionBusy.value = true
  try {
    await proposalApi.accept(proposal.id)
    await refetch()
  } finally {
    actionBusy.value = false
  }
}

async function rejectProposal(proposal: Proposal) {
  actionBusy.value = true
  try {
    await proposalApi.reject(proposal.id)
    await refetch()
  } finally {
    actionBusy.value = false
  }
}

async function acceptChange(proposal: Proposal, change: ProposalChange) {
  changeBusy.value = true
  try {
    await proposalApi.acceptChange(proposal.id, change.id)
    await refetch()
  } finally {
    changeBusy.value = false
  }
}

async function rejectChange(proposal: Proposal, change: ProposalChange) {
  changeBusy.value = true
  try {
    await proposalApi.rejectChange(proposal.id, change.id)
    await refetch()
  } finally {
    changeBusy.value = false
  }
}

async function runValidation(proposal: ProposalVM) {
  validatingId.value = proposal.id
  proposal.validation_error = undefined
  try {
    const result: ValidationResult[] = await validationApi.validateProposal(proposal.id)
    proposal.validation_results = result
  } catch (e) {
    proposal.validation_error = '校验失败：' + (e instanceof Error ? e.message : String(e))
  } finally {
    validatingId.value = null
  }
}

async function refetch() {
  if (!projectId) return
  loading.value = true
  try {
    proposals.value = await proposalApi.list(projectId).catch(() => [])
  } finally {
    loading.value = false
  }
}

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleDateString('zh-CN', { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })
}
</script>

<style scoped>
.proposals-page { height: 100%; overflow-y: auto; padding: var(--space-6) var(--space-8); }
.page-header { margin-bottom: var(--space-6); }
.page-title { font-size: var(--text-2xl); font-weight: 700; font-family: var(--font-serif); }

.loading-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--space-3);
  padding: var(--space-16);
  color: var(--text-tertiary);
}
.loading-icon { font-size: 40px; }
.loading-text { font-size: var(--text-sm); }

.proposal-list { display: flex; flex-direction: column; gap: var(--space-4); }

.proposal-card {
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  background: var(--bg-panel);
  overflow: hidden;
}

.proposal-header {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-4) var(--space-5);
  border-bottom: 1px solid var(--border-muted);
}

.proposal-id {
  font-family: var(--font-mono);
  font-size: var(--text-sm);
  color: var(--text-tertiary);
}

.proposal-time {
  margin-left: auto;
  font-size: var(--text-xs);
  color: var(--text-tertiary);
}

.status-badge {
  font-size: 10px;
  padding: 2px 8px;
  border-radius: 999px;
  flex-shrink: 0;
  font-weight: 600;
}
.status-pending { background: var(--color-warning-subtle); color: var(--color-warning); }
.status-approved { background: var(--color-success-subtle); color: var(--color-success); }
.status-rejected { background: var(--color-error-subtle); color: var(--color-error); }
.status-partiallyaccepted { background: var(--color-info-subtle); color: var(--color-info); }
.status-expired { background: var(--bg-hover); color: var(--text-tertiary); }

.proposal-reason {
  padding: var(--space-3) var(--space-5);
  font-size: var(--text-sm);
  color: var(--text-secondary);
  background: var(--bg-panel-secondary);
}

.reason-label {
  font-weight: 500;
  color: var(--text-primary);
}

.changes-section,
.validation-section {
  padding: var(--space-3) var(--space-5);
}

.section-title {
  font-size: var(--text-xs);
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-tertiary);
  margin-bottom: var(--space-2);
}

.validation-title {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.change-item {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-2) 0;
  border-bottom: 1px solid var(--border-muted);
  font-size: var(--text-sm);
}
.change-item:last-child { border-bottom: none; }

.change-type {
  font-size: 10px;
  padding: 2px 6px;
  border-radius: 3px;
  flex-shrink: 0;
}
.change-type.added { background: var(--color-success-subtle); color: var(--color-success); }
.change-type.modified { background: var(--color-warning-subtle); color: var(--color-warning); }
.change-type.removed { background: var(--color-error-subtle); color: var(--color-error); }

.change-target {
  font-weight: 500;
  min-width: 120px;
}

.change-desc {
  color: var(--text-secondary);
  flex: 1;
}

.change-risk {
  font-size: 10px;
  padding: 2px 6px;
  border-radius: 3px;
}
.change-risk.low { background: var(--color-success-subtle); color: var(--color-success); }
.change-risk.medium { background: var(--color-warning-subtle); color: var(--color-warning); }
.change-risk.high { background: var(--color-error-subtle); color: var(--color-error); }

.change-state {
  flex-basis: 100%;
  display: flex;
  align-items: center;
  gap: var(--space-2);
  font-size: var(--text-xs);
  color: var(--text-secondary);
  padding-left: var(--space-2);
}
.state-key { font-family: var(--font-mono); color: var(--text-primary); }
.state-old { color: var(--text-tertiary); }
.state-arrow { color: var(--text-tertiary); }
.state-new { color: var(--color-info); }

.change-actions { display: flex; gap: var(--space-1); margin-left: var(--space-2); }
.accept-btn, .reject-btn { width: 24px; height: 24px; border: 1px solid var(--border-default); background: transparent; border-radius: var(--radius-sm); cursor: pointer; font-size: var(--text-xs); display: flex; align-items: center; justify-content: center; transition: all var(--transition-fast); }
.accept-btn:hover:not(:disabled) { background: var(--color-success-subtle); border-color: var(--color-success); color: var(--color-success); }
.reject-btn:hover:not(:disabled) { background: var(--color-error-subtle); border-color: var(--color-error); color: var(--color-error); }
.accept-btn:disabled, .reject-btn:disabled { opacity: 0.5; cursor: not-allowed; }

.run-validate-btn {
  padding: 2px var(--space-3);
  border: 1px solid var(--border-default);
  background: transparent;
  color: var(--text-secondary);
  border-radius: var(--radius-sm);
  font-size: var(--text-xs);
  cursor: pointer;
  text-transform: none;
  letter-spacing: 0;
}
.run-validate-btn:hover:not(:disabled) { background: var(--bg-hover); }
.run-validate-btn:disabled { opacity: 0.6; cursor: not-allowed; }

.validation-list { display: flex; flex-direction: column; gap: var(--space-1); }
.validation-item {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-1) 0;
  font-size: var(--text-sm);
}
.vr-severity {
  font-size: 10px;
  padding: 2px 6px;
  border-radius: 3px;
  flex-shrink: 0;
}
.vr-severity.error { background: var(--color-error-subtle); color: var(--color-error); }
.vr-severity.warning { background: var(--color-warning-subtle); color: var(--color-warning); }
.vr-severity.info { background: var(--color-info-subtle); color: var(--color-info); }

.vr-dimension { font-weight: 500; color: var(--text-primary); }
.vr-message { color: var(--text-secondary); flex: 1; }
.vr-suggestion { color: var(--text-tertiary); font-size: var(--text-xs); flex-basis: 100%; }
.validation-empty { font-size: var(--text-sm); color: var(--text-tertiary); }
.validation-error { font-size: var(--text-sm); color: var(--color-error); background: var(--color-error-subtle); padding: var(--space-2) var(--space-3); border-radius: var(--radius-sm); }

.proposal-actions { display: flex; gap: var(--space-2); padding: var(--space-3) var(--space-5); }
.accept-all-btn { padding: var(--space-2) var(--space-4); background: var(--color-success); border: none; color: white; border-radius: var(--radius-sm); font-size: var(--text-sm); cursor: pointer; }
.accept-all-btn:hover:not(:disabled) { opacity: 0.9; }
.reject-all-btn { padding: var(--space-2) var(--space-4); border: 1px solid var(--border-default); background: transparent; color: var(--text-secondary); border-radius: var(--radius-sm); font-size: var(--text-sm); cursor: pointer; }
.reject-all-btn:hover:not(:disabled) { background: var(--bg-hover); }
.accept-all-btn:disabled, .reject-all-btn:disabled { opacity: 0.5; cursor: not-allowed; }

.proposal-footer {
  padding: var(--space-3) var(--space-5);
  border-top: 1px solid var(--border-muted);
  font-size: var(--text-xs);
  color: var(--text-tertiary);
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--space-3);
  padding: var(--space-16) var(--space-8);
  text-align: center;
  color: var(--text-tertiary);
}
.empty-icon { font-size: 48px; }
.empty-title { font-size: var(--text-lg); color: var(--text-secondary); font-weight: 600; }
.empty-desc { font-size: var(--text-sm); max-width: 420px; line-height: 1.6; }

</style>
