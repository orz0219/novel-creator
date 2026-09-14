<template>
  <div class="story-node" :class="[node.node_type, node.status]" @click="$emit('select', node)">
    <div class="node-gutter">
      <span class="expand-icon" v-if="hasChildren" @click.stop="$emit('toggle', node.id)">{{ expanded ? '▼' : '▶' }}</span>
      <span class="expand-icon" v-else>　</span>
    </div>
    <div class="node-body">
      <span class="node-title">{{ node.title }}</span>
      <!-- 挂载信息以 chip 呈现：这些是结构字段，不该再以一段没人读的 JSON 存在 -->
      <span
        v-if="node.storyline_name"
        class="node-chip chip-line"
        :title="`服务故事线：${node.storyline_name}`"
      >{{ node.storyline_name }}</span>
      <span
        v-if="node.arc_stage"
        class="node-chip chip-stage"
        :title="`推进阶段：${node.arc_stage}`"
      >{{ node.arc_stage }}</span>
      <span
        v-if="node.participant_entity_ids && node.participant_entity_ids.length"
        class="node-chip"
        :title="`在场 ${node.participant_entity_ids.length} 个实体`"
      >在场 {{ node.participant_entity_ids.length }}</span>
      <span
        v-if="node.estimated_words"
        class="node-chip"
        :title="`预计 ${node.estimated_words} 字`"
      >约 {{ wordsLabel(node.estimated_words) }}</span>
      <span class="node-status" :class="node.status">{{ statusLabels[node.status] || node.status }}</span>
    </div>
    <div class="node-actions">
      <button v-if="node.node_type === 'Scene'" class="action-btn" @click.stop="$emit('write', node)"><PenLine :size="14" /></button>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { NarrativeNode } from '@/types'
import { PenLine } from 'lucide-vue-next'
defineProps<{
  node: NarrativeNode & { children?: NarrativeNode[] }
  expanded?: boolean
  hasChildren?: boolean
}>()
defineEmits(['select', 'toggle', 'write'])

const statusLabels: Record<string, string> = {
  Draft: '草稿', Planned: '已规划', InProgress: '进行中', Completed: '已完成', Archived: '已归档',
}

/** 字数 chip：万字以上折成「1.2 万字」，避免长数字把树挤变形 */
function wordsLabel(words: number): string {
  if (words >= 10000) return `${(words / 10000).toFixed(1)} 万字`
  return `${words} 字`
}
</script>

<style scoped>
.story-node { display: flex; align-items: center; gap: var(--space-2); padding: var(--space-1) var(--space-2); border-radius: var(--radius-sm); cursor: pointer; transition: background var(--transition-fast); }
.story-node:hover { background: var(--bg-hover); }
.node-gutter { width: 16px; flex-shrink: 0; }
.expand-icon { font-size: 10px; color: var(--text-tertiary); }
.node-body { flex: 1; display: flex; align-items: center; gap: var(--space-2); min-width: 0; }
.node-title { font-size: var(--text-sm); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.node-chip { font-size: 10px; padding: 1px 6px; border-radius: 3px; flex-shrink: 0; background: var(--bg-panel-secondary); color: var(--text-tertiary); max-width: 9em; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.node-chip.chip-line { color: var(--color-accent); background: var(--color-accent-subtle); }
.node-chip.chip-stage { color: var(--text-secondary); }
.node-status { font-size: 10px; padding: 1px 6px; border-radius: 3px; flex-shrink: 0; }
.node-status.Completed { background: var(--color-success-subtle); color: var(--color-success); }
.node-status.InProgress { background: var(--color-accent-subtle); color: var(--color-accent); }
.node-status.Planned { background: var(--bg-panel-secondary); color: var(--text-tertiary); }
.node-status.Draft { background: var(--color-warning-subtle); color: var(--color-warning); }
.node-actions { display: flex; gap: var(--space-1); }
.action-btn { border: none; background: transparent; cursor: pointer; font-size: var(--text-xs); padding: 2px; border-radius: var(--radius-sm); }
.action-btn:hover { background: var(--bg-hover); }
</style>
