<template>
  <div class="proposal-panel">
    <div class="panel-header">
      <span class="panel-title">AI 提案 #{{ proposal.id }}</span>
      <StatusBadge :status="proposal.status" :label="proposal.status" />
    </div>
    <div class="panel-body">
      <div class="proposal-reason" v-if="proposal.reason">{{ proposal.reason }}</div>
      <div class="changes-section">
        <div class="section-title">变更 ({{ proposal.changes.length }})</div>
        <ProposalDiff v-for="change in proposal.changes" :key="change.id" :change="change" @accept="$emit('accept-change', change.id)" @reject="$emit('reject-change', change.id)" />
      </div>
      <div class="validation-section" v-if="proposal.validation_results.length">
        <div class="section-title">验证结果</div>
        <div v-for="vr in proposal.validation_results" :key="vr.id" class="vr-item">
          <span class="vr-severity" :class="vr.severity">{{ vr.severity }}</span>
          <span class="vr-message">{{ vr.message }}</span>
        </div>
      </div>
    </div>
    <div class="panel-footer" v-if="proposal.status === 'Pending'">
      <button class="accept-btn" @click="$emit('accept')">全部接受</button>
      <button class="reject-btn" @click="$emit('reject')">拒绝</button>
    </div>
  </div>
</template>

<script setup lang="ts">
import StatusBadge from '@/components/ui/StatusBadge.vue'
import ProposalDiff from './ProposalDiff.vue'
import type { Proposal } from '@/types'
defineProps<{ proposal: Proposal }>()
defineEmits(['accept', 'reject', 'accept-change', 'reject-change'])
</script>

<style scoped>
.proposal-panel { display: flex; flex-direction: column; }
.panel-header { display: flex; justify-content: space-between; align-items: center; padding: var(--space-3) var(--space-4); border-bottom: 1px solid var(--border-muted); }
.panel-title { font-size: var(--text-sm); font-weight: 600; }
.panel-body { padding: var(--space-3); overflow-y: auto; flex: 1; }
.proposal-reason { font-size: var(--text-sm); color: var(--text-secondary); padding: var(--space-2); background: var(--bg-panel-secondary); border-radius: var(--radius-sm); margin-bottom: var(--space-3); }
.section-title { font-size: var(--text-xs); font-weight: 600; text-transform: uppercase; color: var(--text-tertiary); margin-bottom: var(--space-2); }
.vr-item { display: flex; gap: var(--space-2); padding: var(--space-1) 0; font-size: var(--text-sm); }
.vr-severity { font-size: 10px; padding: 2px 6px; border-radius: 3px; }
.vr-severity.Error { background: var(--color-error-subtle); color: var(--color-error); }
.vr-severity.Warning { background: var(--color-warning-subtle); color: var(--color-warning); }
.vr-severity.Info { background: var(--color-info-subtle); color: var(--color-info); }
.vr-message { color: var(--text-secondary); }
.panel-footer { display: flex; gap: var(--space-2); padding: var(--space-3) var(--space-4); border-top: 1px solid var(--border-muted); }
.accept-btn { flex: 1; padding: var(--space-2); background: var(--color-success); border: none; color: white; border-radius: var(--radius-sm); font-size: var(--text-sm); cursor: pointer; }
.accept-btn:hover { opacity: 0.9; }
.reject-btn { flex: 1; padding: var(--space-2); border: 1px solid var(--color-error); background: transparent; color: var(--color-error); border-radius: var(--radius-sm); font-size: var(--text-sm); cursor: pointer; }
.reject-btn:hover { background: var(--color-error-subtle); }
</style>
