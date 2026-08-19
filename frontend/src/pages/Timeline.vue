<template>
  <div class="timeline-page">
    <div class="page-header">
      <h1 class="page-title">时间线</h1>
    </div>
    <div class="timeline">
      <div v-for="event in worldStore.events" :key="event.id" class="timeline-item">
        <div class="timeline-marker"></div>
        <div class="timeline-content">
          <div class="timeline-time">{{ event.timestamp }}</div>
          <div class="timeline-name">{{ event.name }}</div>
          <div class="timeline-desc">{{ event.description }}</div>
          <div class="timeline-entities">
            <span v-for="eid in event.involved_entity_ids" :key="eid" class="entity-tag">
              {{ getEntityName(eid) }}
            </span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useWorldStore } from '@/stores/world'
const worldStore = useWorldStore()

function getEntityName(id: string): string {
  const all = [...worldStore.characters, ...worldStore.locations, ...worldStore.factions]
  return all.find(e => e.id === id)?.name || id
}
</script>

<style scoped>
.timeline-page { height: 100%; overflow-y: auto; padding: var(--space-6) var(--space-8); }
.page-header { margin-bottom: var(--space-6); }
.page-title { font-size: var(--text-2xl); font-weight: 700; font-family: var(--font-serif); }

.timeline { position: relative; padding-left: var(--space-8); }
.timeline::before { content: ''; position: absolute; left: 12px; top: 0; bottom: 0; width: 2px; background: var(--border-default); }

.timeline-item { position: relative; margin-bottom: var(--space-6); }
.timeline-marker { position: absolute; left: calc(-1 * var(--space-8) + 8px); top: 4px; width: 10px; height: 10px; border-radius: 50%; background: var(--color-primary); border: 2px solid var(--bg-base); }
.timeline-content { padding: var(--space-4); border: 1px solid var(--border-default); border-radius: var(--radius-md); background: var(--bg-panel); }
.timeline-time { font-size: var(--text-xs); color: var(--text-tertiary); margin-bottom: var(--space-1); }
.timeline-name { font-size: var(--text-md); font-weight: 600; margin-bottom: var(--space-2); }
.timeline-desc { font-size: var(--text-sm); color: var(--text-secondary); margin-bottom: var(--space-3); }
.timeline-entities { display: flex; gap: var(--space-2); flex-wrap: wrap; }
.entity-tag { font-size: var(--text-xs); padding: 2px 8px; background: var(--bg-panel-secondary); border-radius: 10px; color: var(--text-secondary); }
</style>
