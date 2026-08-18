<template>
  <div class="character-card" @click="$emit('click')">
    <div class="card-avatar">{{ entity.name[0] }}</div>
    <div class="card-body">
      <div class="card-header">
        <span class="card-name">{{ entity.name }}</span>
        <span class="card-version">v{{ entity.version }}</span>
      </div>
      <div class="card-identity" v-if="entity.attributes?.identity">{{ entity.attributes.identity }}</div>
      <div class="card-summary" v-if="entity.summary">{{ entity.summary }}</div>
      <div class="card-stats">
        <span v-if="entity.attributes?.cultivation" class="stat">⚡ {{ entity.attributes.cultivation }}</span>
        <span v-if="entity.attributes?.location" class="stat">📍 {{ entity.attributes.location }}</span>
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
.character-card { display: flex; gap: var(--space-3); padding: var(--space-4); border: 1px solid var(--border-default); border-radius: var(--radius-md); background: var(--bg-panel); cursor: pointer; transition: all var(--transition-fast); }
.character-card:hover { border-color: var(--border-emphasis); background: var(--bg-panel-secondary); }
.card-avatar { width: 40px; height: 40px; border-radius: 50%; background: var(--color-primary-subtle); color: var(--color-primary-text); display: flex; align-items: center; justify-content: center; font-weight: 600; flex-shrink: 0; }
.card-body { flex: 1; min-width: 0; }
.card-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--space-1); }
.card-name { font-weight: 600; font-size: var(--text-md); }
.card-version { font-size: var(--text-xs); color: var(--text-tertiary); font-family: var(--font-mono); }
.card-identity { font-size: var(--text-xs); color: var(--color-primary-text); margin-bottom: var(--space-1); }
.card-summary { font-size: var(--text-sm); color: var(--text-secondary); margin-bottom: var(--space-2); display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
.card-stats { display: flex; gap: var(--space-3); }
.stat { font-size: var(--text-xs); color: var(--text-tertiary); }
</style>
