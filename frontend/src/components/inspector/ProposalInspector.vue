<template>
  <InspectorPanel entity-type="Proposal" :entity-name="'#' + proposal.id" :tabs="tabs" @close="$emit('close')">
    <template #default="{ activeTab }">
      <div v-if="activeTab === 'overview'" class="section">
        <div class="info-grid">
          <div class="info-row"><span class="label">状态</span><StatusBadge :status="proposal.status" :label="proposal.status" /></div>
          <div class="info-row"><span class="label">变更数</span><span>{{ proposal.changes.length }}</span></div>
          <div class="info-row"><span class="label">创建时间</span><span>{{ proposal.created_at }}</span></div>
        </div>
        <div class="reason" v-if="proposal.reason"><div class="section-label">原因</div><p>{{ proposal.reason }}</p></div>
      </div>
      <div v-if="activeTab === 'changes'" class="section">
        <div class="section-label">变更列表</div>
        <div v-for="change in proposal.changes" :key="change.id" class="change-item">
          <span class="change-type" :class="change.change_type">{{ change.change_type }}</span>
          <span class="change-target">{{ change.target_entity_name }}</span>
          <span class="change-desc">{{ change.description }}</span>
        </div>
      </div>
      <div v-if="activeTab === 'validation'" class="section">
        <div class="section-label">验证结果</div>
        <div v-for="vr in proposal.validation_results" :key="vr.id" class="vr-item">
          <span class="vr-severity" :class="vr.severity">{{ vr.severity }}</span>
          <span class="vr-message">{{ vr.message }}</span>
        </div>
      </div>
    </template>
  </InspectorPanel>
</template>

<script setup lang="ts">
import InspectorPanel from './InspectorPanel.vue'
import StatusBadge from '@/components/ui/StatusBadge.vue'
import type { Proposal } from '@/types'
defineProps<{ proposal: Proposal }>()
defineEmits(['close'])
const tabs = [
  { id: 'overview', label: '概览' },
  { id: 'changes', label: '变更' },
  { id: 'validation', label: '验证' },
]
</script>

<style scoped>
.section { margin-bottom: var(--space-4); }
.section-label { font-size: var(--text-xs); font-weight: 600; text-transform: uppercase; color: var(--text-tertiary); margin-bottom: var(--space-2); }
.info-grid { display: flex; flex-direction: column; gap: var(--space-2); }
.info-row { display: flex; justify-content: space-between; align-items: center; font-size: var(--text-sm); }
.label { color: var(--text-tertiary); }
.reason p { font-size: var(--text-sm); color: var(--text-secondary); }
.change-item { display: flex; gap: var(--space-2); padding: var(--space-2) 0; border-bottom: 1px solid var(--border-muted); font-size: var(--text-sm); }
.change-type { font-size: 10px; padding: 2px 6px; border-radius: 3px; }
.change-type.Added { background: var(--color-success-subtle); color: var(--color-success); }
.change-type.Modified { background: var(--color-warning-subtle); color: var(--color-warning); }
.change-target { font-weight: 500; }
.change-desc { color: var(--text-secondary); }
.vr-item { display: flex; gap: var(--space-2); padding: var(--space-2) 0; font-size: var(--text-sm); }
.vr-severity { font-size: 10px; padding: 2px 6px; border-radius: 3px; }
.vr-severity.Error { background: var(--color-error-subtle); color: var(--color-error); }
.vr-severity.Warning { background: var(--color-warning-subtle); color: var(--color-warning); }
.vr-severity.Info { background: var(--color-info-subtle); color: var(--color-info); }
.vr-message { color: var(--text-secondary); }
</style>
