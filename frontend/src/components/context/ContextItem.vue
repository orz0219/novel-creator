<template>
  <div class="context-item" :class="{ pinned: item.policy === 'Pinned', excluded: item.policy === 'Excluded' }">
    <div class="item-header">
      <span class="item-type">{{ item.entity_type }}</span>
      <span class="item-name">{{ item.entity_name }}</span>
      <span class="item-relevance">{{ Math.round(item.relevance * 100) }}%</span>
    </div>
    <div class="item-reasons">
      <div v-for="reason in item.reasons" :key="reason" class="reason">✓ {{ reason }}</div>
    </div>
    <div class="item-actions">
      <button class="ctx-btn" :class="{ active: item.policy === 'Pinned' }" @click="$emit('pin')" title="钉住">📌</button>
      <button class="ctx-btn" :class="{ active: item.policy === 'Excluded' }" @click="$emit('exclude')" title="排除">🚫</button>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { ContextEntity } from '@/types'
defineProps<{ item: ContextEntity }>()
defineEmits(['pin', 'exclude'])
</script>

<style scoped>
.context-item { padding: var(--space-2); border: 1px solid var(--border-muted); border-radius: var(--radius-sm); margin-bottom: var(--space-2); transition: all var(--transition-fast); }
.context-item.pinned { border-color: var(--color-accent); background: var(--color-accent-subtle); }
.context-item.excluded { opacity: 0.5; border-color: var(--color-error); }
.item-header { display: flex; align-items: center; gap: var(--space-2); margin-bottom: var(--space-1); }
.item-type { font-size: 10px; padding: 1px 4px; border-radius: 2px; background: var(--bg-panel-secondary); color: var(--text-tertiary); }
.item-name { font-size: var(--text-sm); font-weight: 500; }
.item-relevance { margin-left: auto; font-size: var(--text-xs); color: var(--text-tertiary); }
.item-reasons { margin-bottom: var(--space-1); }
.reason { font-size: var(--text-xs); color: var(--text-secondary); padding: 1px 0; }
.item-actions { display: flex; gap: var(--space-1); }
.ctx-btn { padding: 2px 4px; border: 1px solid var(--border-muted); background: transparent; border-radius: var(--radius-sm); cursor: pointer; font-size: 10px; transition: all var(--transition-fast); }
.ctx-btn:hover { background: var(--bg-hover); }
.ctx-btn.active { background: var(--bg-active); border-color: var(--border-emphasis); }
</style>
