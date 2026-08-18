<template>
  <div class="graph-controls">
    <div class="control-group">
      <button class="control-btn" @click="$emit('zoom-in')" title="放大">+</button>
      <button class="control-btn" @click="$emit('zoom-out')" title="缩小">-</button>
      <button class="control-btn" @click="$emit('zoom-reset')" title="重置">⟲</button>
    </div>
    <div class="control-separator"></div>
    <div class="control-group">
      <button class="control-btn" @click="$emit('fit')" title="适应画面">⊡</button>
      <button class="control-btn" @click="$emit('center')" title="居中">◎</button>
    </div>
    <div class="control-separator"></div>
    <div class="control-group">
      <button
        v-for="filter in filters"
        :key="filter.id"
        class="control-btn filter"
        :class="{ active: activeFilter === filter.id }"
        @click="$emit('filter', filter.id)"
      >{{ filter.icon }}</button>
    </div>
    <div class="control-info">
      <span class="zoom-level">{{ Math.round(zoom * 100) }}%</span>
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  zoom: number
  activeFilter: string
}>()

defineEmits(['zoom-in', 'zoom-out', 'zoom-reset', 'fit', 'center', 'filter'])

const filters = [
  { id: 'all', icon: '🌐' },
  { id: 'Character', icon: '👤' },
  { id: 'Location', icon: '📍' },
  { id: 'Faction', icon: '⚔️' },
  { id: 'Event', icon: '📅' },
  { id: 'Thread', icon: '🧵' },
]
</script>

<style scoped>
.graph-controls {
  position: absolute; bottom: var(--space-4); left: var(--space-4);
  display: flex; align-items: center; gap: var(--space-2);
  background: var(--bg-panel); border: 1px solid var(--border-default);
  border-radius: var(--radius-md); padding: var(--space-2); box-shadow: var(--shadow-md);
}
.control-group { display: flex; gap: var(--space-1); }
.control-btn {
  width: 28px; height: 28px; display: flex; align-items: center; justify-content: center;
  border: 1px solid var(--border-muted); background: transparent; color: var(--text-secondary);
  border-radius: var(--radius-sm); cursor: pointer; font-size: var(--text-sm);
  transition: all var(--transition-fast);
}
.control-btn:hover { background: var(--bg-hover); color: var(--text-primary); }
.control-btn.active { background: var(--color-primary-subtle); border-color: var(--color-primary); color: var(--color-primary-text); }
.control-separator { width: 1px; height: 20px; background: var(--border-muted); }
.control-info { margin-left: var(--space-2); }
.zoom-level { font-size: var(--text-xs); color: var(--text-tertiary); }
</style>
