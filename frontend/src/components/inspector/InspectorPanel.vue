<template>
  <div class="inspector-panel">
    <div class="inspector-header">
      <span class="inspector-type-badge">{{ entityType }}</span>
      <span class="inspector-name">{{ entityName }}</span>
      <button class="close-btn" @click="$emit('close')">×</button>
    </div>
    <div class="inspector-tabs">
      <button
        v-for="tab in availableTabs"
        :key="tab.id"
        class="inspector-tab"
        :class="{ active: activeTab === tab.id }"
        @click="activeTab = tab.id"
      >{{ tab.label }}</button>
    </div>
    <div class="inspector-content">
      <slot :activeTab="activeTab" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
const props = defineProps<{
  entityType: string
  entityName: string
  tabs?: { id: string; label: string }[]
}>()
defineEmits(['close'])
const activeTab = ref('overview')
const availableTabs = computed(() => props.tabs || [
  { id: 'overview', label: '概览' },
  { id: 'state', label: '状态' },
  { id: 'relations', label: '关系' },
  { id: 'history', label: '历史' },
])
</script>

<style scoped>
.inspector-panel {
  display: flex; flex-direction: column; height: 100%;
  background: var(--bg-panel); border-left: 1px solid var(--border-default);
}
.inspector-header {
  display: flex; align-items: center; gap: var(--space-2);
  padding: var(--space-3) var(--space-4); border-bottom: 1px solid var(--border-muted);
}
.inspector-type-badge {
  font-size: 10px; padding: 2px 6px; border-radius: 3px;
  background: var(--bg-panel-secondary); color: var(--text-tertiary);
}
.inspector-name { font-weight: 600; font-size: var(--text-md); }
.close-btn {
  margin-left: auto; border: none; background: transparent;
  color: var(--text-tertiary); font-size: var(--text-xl); cursor: pointer;
}
.close-btn:hover { color: var(--text-primary); }
.inspector-tabs {
  display: flex; border-bottom: 1px solid var(--border-muted); flex-shrink: 0;
}
.inspector-tab {
  flex: 1; padding: var(--space-2); border: none; background: transparent;
  color: var(--text-tertiary); font-size: var(--text-xs); cursor: pointer;
  border-bottom: 2px solid transparent; transition: all var(--transition-fast);
  font-family: inherit;
}
.inspector-tab:hover { color: var(--text-secondary); }
.inspector-tab.active { color: var(--text-primary); border-bottom-color: var(--color-primary); }
.inspector-content { flex: 1; overflow-y: auto; padding: var(--space-3) var(--space-4); }
</style>
