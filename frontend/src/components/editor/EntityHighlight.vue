<template>
  <span class="entity-highlight" :class="entityType" @click="openInspector" :title="entityName">
    <slot />
    <span class="entity-indicator">{{ typeIcon }}</span>
  </span>
</template>

<script setup lang="ts">
import { computed } from 'vue'
const props = defineProps<{
  entityId: string
  entityType: 'Character' | 'Location' | 'Faction' | 'Item'
  entityName: string
}>()

const typeIcon = computed(() => {
  const icons: Record<string, string> = { Character: '👤', Location: '📍', Faction: '⚔️', Item: '📦' }
  return icons[props.entityType] || '📌'
})

function openInspector() {
  // Emit event to parent to open inspector
  console.log('Open inspector for:', props.entityId, props.entityType)
}
</script>

<style scoped>
.entity-highlight {
  cursor: pointer; border-bottom: 1px dashed; transition: all var(--transition-fast);
  position: relative; display: inline;
}
.entity-highlight:hover { opacity: 0.8; }
.entity-indicator { font-size: 10px; margin-left: 2px; vertical-align: super; }
.entity-highlight.Character { border-color: var(--color-accent); color: var(--color-accent); }
.entity-highlight.Location { border-color: var(--color-success); color: var(--color-success); }
.entity-highlight.Faction { border-color: var(--color-warning); color: var(--color-warning); }
.entity-highlight.Item { border-color: var(--text-tertiary); color: var(--text-tertiary); }
</style>
