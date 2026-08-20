<template>
  <div class="entity-card" @click="$emit('click')">
    <div class="card-header">
      <span class="card-type">{{ type }}</span>
      <button class="card-delete" title="删除" @click.stop="$emit('delete')">🗑</button>
      <span class="card-version">v{{ entity.version }}</span>
    </div>
    <div class="card-name">{{ entity.name }}</div>
    <div class="card-summary" v-if="entity.summary">{{ entity.summary }}</div>
    <div class="card-meta">
      <span class="meta-item" v-if="entity.source_generation_id">🤖 AI 生成</span>
      <span class="meta-item">{{ formatDate(entity.updated_at) }}</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { Entity } from '@/types'

defineProps<{
  entity: Entity
  type: string
}>()

defineEmits(['click', 'delete'])

function formatDate(dateStr: string): string {
  const date = new Date(dateStr)
  return date.toLocaleDateString('zh-CN', { month: 'short', day: 'numeric' })
}
</script>

<style scoped>
.entity-card {
  padding: var(--space-4);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  background: var(--bg-panel);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.entity-card:hover {
  border-color: var(--border-emphasis);
  background: var(--bg-panel-secondary);
}

.card-delete {
  opacity: 0;
  border: none;
  background: transparent;
  cursor: pointer;
  font-size: 13px;
  line-height: 1;
  padding: 2px 4px;
  border-radius: 4px;
  transition: opacity var(--transition-fast);
}
.entity-card:hover .card-delete {
  opacity: 1;
}
.card-delete:hover {
  background: var(--color-error-subtle);
}

.card-header {
  display: flex;
  justify-content: space-between;
  margin-bottom: var(--space-2);
}

.card-type {
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-tertiary);
  padding: 1px 6px;
  background: var(--bg-panel-secondary);
  border-radius: 3px;
}

.card-version {
  font-size: var(--text-xs);
  color: var(--text-tertiary);
}

.card-name {
  font-size: var(--text-md);
  font-weight: 500;
  margin-bottom: var(--space-1);
}

.card-summary {
  font-size: var(--text-sm);
  color: var(--text-secondary);
  margin-bottom: var(--space-2);
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.card-meta {
  display: flex;
  gap: var(--space-3);
  font-size: var(--text-xs);
  color: var(--text-tertiary);
}
</style>
