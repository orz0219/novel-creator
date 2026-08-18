<template>
  <div class="faction-card" @click="$emit('click')">
    <div class="card-icon">⚔️</div>
    <div class="card-body">
      <div class="card-header">
        <span class="card-name">{{ entity.name }}</span>
        <span class="card-version">v{{ entity.version }}</span>
      </div>
      <div class="card-summary" v-if="entity.summary">{{ entity.summary }}</div>
      <div class="card-meta">
        <span v-if="entity.attributes?.leader" class="meta">👑 {{ entity.attributes.leader }}</span>
        <span v-if="entity.attributes?.territory" class="meta">🗺️ {{ entity.attributes.territory }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { Entity } from '@/types'
defineProps<{ entity: Entity }>()
defineEmits(['click'])
</script>

<style scoped>
.faction-card { display: flex; gap: var(--space-3); padding: var(--space-4); border: 1px solid var(--border-default); border-radius: var(--radius-md); background: var(--bg-panel); cursor: pointer; transition: all var(--transition-fast); }
.faction-card:hover { border-color: var(--border-emphasis); background: var(--bg-panel-secondary); }
.card-icon { font-size: 24px; flex-shrink: 0; }
.card-body { flex: 1; min-width: 0; }
.card-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--space-1); }
.card-name { font-weight: 600; font-size: var(--text-md); }
.card-version { font-size: var(--text-xs); color: var(--text-tertiary); font-family: var(--font-mono); }
.card-summary { font-size: var(--text-sm); color: var(--text-secondary); margin-bottom: var(--space-2); }
.card-meta { display: flex; gap: var(--space-3); }
.meta { font-size: var(--text-xs); color: var(--text-tertiary); }
</style>
