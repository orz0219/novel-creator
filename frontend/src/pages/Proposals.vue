<template>
  <div class="proposals-page">
    <div class="page-header">
      <h1 class="page-title">AI 提案</h1>
    </div>

    <div class="proposal-list">
      <div v-for="proposal in proposalStore.proposals" :key="proposal.id" class="proposal-card">        <div class="proposal-header">
          <span class="proposal-id">#{{ proposal.id.split('-')[1] }}</span>
          <StatusBadge :status="(proposal.status || '').toLowerCase()" :label="proposal.status" />
          <span class="proposal-time">{{ formatDate(proposal.created_at) }}</span>
        </div>
        <div class="proposal-reason" v-if="proposal.reason">
          <span class="reason-label">原因：</span>{{ proposal.reason }}
        </div>

        <!-- Changes -->
        <div class="changes-section">
          <div class="section-title">变更 ({{ proposal.changes.length }})</div>
          <div v-for="change in proposal.changes" :key="change.id" class="change-item">
            <span class="change-type" :class="change.change_type.toLowerCase()">{{ changeTypeLabels[change.change_type] }}</span>
            <span class="change-target">{{ change.target_entity_type }}: {{ change.target_entity_name }}</span>
            <span class="change-desc">{{ change.description }}</span>
            <span class="change-risk" :class="change.risk_level.toLowerCase()">{{ change.risk_level }}</span>
            <div class="change-actions" v-if="proposal.status === 'Pending'">
              <button class="accept-btn" @click.stop="acceptChange(change)">✓</button>
              <button class="reject-btn" @click.stop="rejectChange(change)">✗</button>
            </div>
          </div>
        </div>

        <!-- Validation -->
        <div class="validation-section">
          <div class="section-title">验证结果</div>
          <div v-for="vr in proposal.validation_results" :key="vr.id" class="validation-item">
            <span class="vr-severity" :class="vr.severity.toLowerCase()">{{ vr.severity }}</span>
            <span class="vr-message">{{ vr.message }}</span>
          </div>
        </div>

        <div class="proposal-actions" v-if="proposal.status === 'Pending'">
          <button class="accept-all-btn" @click="acceptAll(proposal)">全部接受</button>
          <button class="review-btn" @click="reviewMode = proposal.id">逐项审核</button>
        </div>
        <div class="proposal-footer">
          <span class="review-time" v-if="proposal.reviewed_at">
            审核于 {{ formatDate(proposal.reviewed_at) }}
          </span>
        </div>
      </div>
    </div>

    <div v-if="proposalStore.proposals.length === 0 && !proposalStore.loading" class="empty-state">
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
import { useProposalStore } from '@/stores/proposal'
import StatusBadge from '@/components/ui/StatusBadge.vue'

import { ref, onMounted } from "vue"
import { useRoute } from 'vue-router'
const reviewMode = ref<string | null>(null)

const route = useRoute()
const proposalStore = useProposalStore()

onMounted(() => {
  const projectId = route.params.id as string
  if (projectId) proposalStore.fetchProposals(projectId)
})

const changeTypeLabels: Record<string, string> = {
  Added: '新增',
  Removed: '删除',
  Modified: '修改',
}

function acceptChange(change: any) { change.accepted = true; }
function rejectChange(change: any) { change.accepted = false; }

function acceptAll(proposal: any) { proposal.changes.forEach((c: any) => c.accepted = true); }

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleDateString('zh-CN', { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })
}
</script>

<style scoped>
.proposals-page { height: 100%; overflow-y: auto; padding: var(--space-6) var(--space-8); }
.page-header { margin-bottom: var(--space-6); }
.page-title { font-size: var(--text-2xl); font-weight: 700; font-family: var(--font-serif); }

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

.change-item {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-2) 0;
  border-bottom: 1px solid var(--border-muted);
  font-size: var(--text-sm);
}

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

.validation-item {
  display: flex;
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

.vr-message { color: var(--text-secondary); }

.change-actions { display: flex; gap: var(--space-1); margin-left: var(--space-2); }
.accept-btn, .reject-btn { width: 24px; height: 24px; border: 1px solid var(--border-default); background: transparent; border-radius: var(--radius-sm); cursor: pointer; font-size: var(--text-xs); display: flex; align-items: center; justify-content: center; transition: all var(--transition-fast); }
.accept-btn:hover { background: var(--color-success-subtle); border-color: var(--color-success); color: var(--color-success); }
.reject-btn:hover { background: var(--color-error-subtle); border-color: var(--color-error); color: var(--color-error); }

.proposal-actions { display: flex; gap: var(--space-2); padding: var(--space-3) var(--space-5); }
.accept-all-btn { padding: var(--space-2) var(--space-4); background: var(--color-success); border: none; color: white; border-radius: var(--radius-sm); font-size: var(--text-sm); cursor: pointer; }
.accept-all-btn:hover { opacity: 0.9; }
.review-btn { padding: var(--space-2) var(--space-4); border: 1px solid var(--border-default); background: transparent; color: var(--text-secondary); border-radius: var(--radius-sm); font-size: var(--text-sm); cursor: pointer; }
.review-btn:hover { background: var(--bg-hover); }

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