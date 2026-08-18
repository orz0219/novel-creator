<template>
  <div class="proposal-diff">
    <div class="diff-header">
      <span class="diff-type" :class="change.change_type">{{ typeLabels[change.change_type] }}</span>
      <span class="diff-target">{{ change.target_entity_type }}: {{ change.target_entity_name }}</span>
      <span class="diff-risk" :class="change.risk_level">{{ change.risk_level }}</span>
    </div>
    <div class="diff-desc">{{ change.description }}</div>
    <div class="diff-actions" v-if="showActions">
      <button class="accept-btn" @click="$emit('accept')">✓</button>
      <button class="reject-btn" @click="$emit('reject')">✗</button>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { ProposalChange } from '@/types'
defineProps<{
  change: ProposalChange
  showActions?: boolean
}>()
defineEmits(['accept', 'reject'])
const typeLabels: Record<string, string> = { Added: '新增', Removed: '删除', Modified: '修改' }
</script>

<style scoped>
.proposal-diff { padding: var(--space-2); border: 1px solid var(--border-muted); border-radius: var(--radius-sm); margin-bottom: var(--space-2); }
.diff-header { display: flex; align-items: center; gap: var(--space-2); margin-bottom: var(--space-1); }
.diff-type { font-size: 10px; padding: 2px 6px; border-radius: 3px; }
.diff-type.Added { background: var(--color-success-subtle); color: var(--color-success); }
.diff-type.Modified { background: var(--color-warning-subtle); color: var(--color-warning); }
.diff-type.Removed { background: var(--color-error-subtle); color: var(--color-error); }
.diff-target { font-size: var(--text-sm); font-weight: 500; }
.diff-risk { margin-left: auto; font-size: 10px; padding: 2px 6px; border-radius: 3px; }
.diff-risk.Low { background: var(--color-success-subtle); color: var(--color-success); }
.diff-risk.Medium { background: var(--color-warning-subtle); color: var(--color-warning); }
.diff-risk.High { background: var(--color-error-subtle); color: var(--color-error); }
.diff-desc { font-size: var(--text-xs); color: var(--text-secondary); margin-bottom: var(--space-1); }
.diff-actions { display: flex; gap: var(--space-1); }
.accept-btn, .reject-btn { width: 20px; height: 20px; border: 1px solid var(--border-muted); background: transparent; border-radius: var(--radius-sm); cursor: pointer; font-size: 10px; display: flex; align-items: center; justify-content: center; }
.accept-btn:hover { background: var(--color-success-subtle); border-color: var(--color-success); }
.reject-btn:hover { background: var(--color-error-subtle); border-color: var(--color-error); }
</style>
