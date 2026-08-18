<template>
  <div class="context-panel">
    <div class="panel-header">
      <span class="panel-title">Context</span>
      <span class="panel-tokens">{{ totalTokens.toLocaleString() }} tokens</span>
    </div>
    <div class="panel-body">
      <div class="section">
        <div class="section-title">实体 ({{ entities.length }})</div>
        <ContextItem v-for="entity in entities" :key="entity.entity_id" :item="entity" @pin="$emit('pin', entity.entity_id)" @exclude="$emit('exclude', entity.entity_id)" />
      </div>
      <div class="section">
        <div class="section-title">项目 ({{ items.length }})</div>
        <div v-for="item in items" :key="item.id" class="context-meta">
          <span class="meta-type">{{ item.type }}</span>
          <span class="meta-content">{{ item.content }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import ContextItem from './ContextItem.vue'
import type { ContextEntity, ContextItem as CI } from '@/types'
defineProps<{
  entities: ContextEntity[]
  items: CI[]
  totalTokens: number
}>()
defineEmits(['pin', 'exclude'])
</script>

<style scoped>
.context-panel { display: flex; flex-direction: column; }
.panel-header { display: flex; justify-content: space-between; align-items: center; padding: var(--space-3) var(--space-4); border-bottom: 1px solid var(--border-muted); }
.panel-title { font-size: var(--text-sm); font-weight: 600; }
.panel-tokens { font-size: var(--text-xs); color: var(--text-tertiary); background: var(--bg-panel-secondary); padding: 2px 8px; border-radius: 10px; }
.panel-body { padding: var(--space-3); overflow-y: auto; }
.section { margin-bottom: var(--space-4); }
.section-title { font-size: var(--text-xs); font-weight: 600; text-transform: uppercase; color: var(--text-tertiary); margin-bottom: var(--space-2); }
.context-meta { display: flex; gap: var(--space-2); padding: var(--space-1) 0; font-size: var(--text-xs); }
.meta-type { color: var(--text-tertiary); text-transform: uppercase; min-width: 60px; }
.meta-content { color: var(--text-secondary); }
</style>
