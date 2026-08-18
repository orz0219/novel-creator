<template>
  <InspectorPanel entity-type="Generation" :entity-name="'任务 #' + task.id" :tabs="tabs" @close="$emit('close')">
    <template #default="{ activeTab }">
      <div v-if="activeTab === 'overview'" class="section">
        <div class="info-grid">
          <div class="info-row"><span class="label">类型</span><span>{{ task.type }}</span></div>
          <div class="info-row"><span class="label">状态</span><StatusBadge :status="task.status" :label="task.status" /></div>
          <div class="info-row"><span class="label">模型</span><span>{{ task.model }}</span></div>
          <div class="info-row"><span class="label">Context Tokens</span><span>{{ task.context_tokens?.toLocaleString() || '-' }}</span></div>
        </div>
        <div class="result" v-if="task.result"><div class="section-label">结果</div><p>{{ task.result }}</p></div>
      </div>
      <div v-if="activeTab === 'context'" class="section">
        <div class="section-label">使用的 Context</div>
        <div class="ctx-item" v-for="item in mockContext" :key="item.id">
          <span class="ctx-type">{{ item.type }}</span>
          <span class="ctx-content">{{ item.content }}</span>
        </div>
      </div>
      <div v-if="activeTab === 'params'" class="section">
        <div class="section-label">参数</div>
        <div class="param-item" v-for="(value, key) in task.parameters" :key="key">
          <span class="param-key">{{ key }}</span>
          <span class="param-value">{{ value }}</span>
        </div>
      </div>
    </template>
  </InspectorPanel>
</template>

<script setup lang="ts">
import InspectorPanel from './InspectorPanel.vue'
import StatusBadge from '@/components/ui/StatusBadge.vue'
import type { GenerationTask } from '@/types'
defineProps<{ task: GenerationTask }>()
defineEmits(['close'])
const tabs = [
  { id: 'overview', label: '概览' },
  { id: 'context', label: 'Context' },
  { id: 'params', label: '参数' },
]
const mockContext = [
  { id: '1', type: 'Entity', content: '林凡 (Character)' },
  { id: '2', type: 'Entity', content: '地下遗迹 (Location)' },
  { id: '3', type: 'Timeline', content: '天玄历381年3月12日' },
  { id: '4', type: 'Constraint', content: '遗迹内可能有远古阵法' },
]
</script>

<style scoped>
.section { margin-bottom: var(--space-4); }
.section-label { font-size: var(--text-xs); font-weight: 600; text-transform: uppercase; color: var(--text-tertiary); margin-bottom: var(--space-2); }
.info-grid { display: flex; flex-direction: column; gap: var(--space-2); }
.info-row { display: flex; justify-content: space-between; align-items: center; font-size: var(--text-sm); }
.label { color: var(--text-tertiary); }
.result p { font-size: var(--text-sm); color: var(--text-secondary); }
.ctx-item, .param-item { display: flex; gap: var(--space-3); padding: var(--space-2) 0; border-bottom: 1px solid var(--border-muted); font-size: var(--text-sm); }
.ctx-type { font-size: 10px; padding: 2px 6px; border-radius: 3px; background: var(--bg-panel-secondary); color: var(--text-tertiary); min-width: 60px; text-align: center; }
.ctx-content { color: var(--text-secondary); }
.param-key { font-family: var(--font-mono); color: var(--text-tertiary); min-width: 80px; }
.param-value { color: var(--text-secondary); }
</style>
