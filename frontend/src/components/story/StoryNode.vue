<template>
  <div class="story-node" :class="[node.node_type, node.status]" @click="$emit('select', node)">
    <div class="node-gutter">
      <span class="expand-icon" v-if="hasChildren" @click.stop="$emit('toggle', node.id)">{{ expanded ? '▼' : '▶' }}</span>
      <span class="expand-icon" v-else>　</span>
    </div>
    <div class="node-body">
      <span class="node-title">{{ node.title }}</span>
      <span class="node-status" :class="node.status">{{ statusLabels[node.status] || node.status }}</span>
    </div>
    <div class="node-actions">
      <button v-if="node.node_type === 'Scene'" class="action-btn" @click.stop="$emit('write', node)">✍️</button>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { NarrativeNode } from '@/types'
defineProps<{
  node: NarrativeNode & { children?: NarrativeNode[] }
  expanded?: boolean
  hasChildren?: boolean
}>()
defineEmits(['select', 'toggle', 'write'])

const statusLabels: Record<string, string> = {
  Draft: '草稿', Planned: '已规划', InProgress: '进行中', Completed: '已完成', Archived: '已归档',
}
</script>

<style scoped>
.story-node { display: flex; align-items: center; gap: var(--space-2); padding: var(--space-1) var(--space-2); border-radius: var(--radius-sm); cursor: pointer; transition: background var(--transition-fast); }
.story-node:hover { background: var(--bg-hover); }
.node-gutter { width: 16px; flex-shrink: 0; }
.expand-icon { font-size: 10px; color: var(--text-tertiary); }
.node-body { flex: 1; display: flex; align-items: center; gap: var(--space-2); min-width: 0; }
.node-title { font-size: var(--text-sm); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.node-status { font-size: 10px; padding: 1px 6px; border-radius: 3px; flex-shrink: 0; }
.node-status.Completed { background: var(--color-success-subtle); color: var(--color-success); }
.node-status.InProgress { background: var(--color-accent-subtle); color: var(--color-accent); }
.node-status.Planned { background: var(--bg-panel-secondary); color: var(--text-tertiary); }
.node-status.Draft { background: var(--color-warning-subtle); color: var(--color-warning); }
.node-actions { display: flex; gap: var(--space-1); }
.action-btn { border: none; background: transparent; cursor: pointer; font-size: var(--text-xs); padding: 2px; border-radius: var(--radius-sm); }
.action-btn:hover { background: var(--bg-hover); }
</style>
