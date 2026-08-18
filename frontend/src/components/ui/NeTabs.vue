<template>
  <div class="ne-tabs">
    <div class="tabs-header">
      <button
        v-for="tab in tabs"
        :key="tab.id"
        class="tab-btn"
        :class="{ active: modelValue === tab.id }"
        @click="$emit('update:modelValue', tab.id)"
      >
        {{ tab.label }}
        <span class="tab-badge" v-if="tab.badge">{{ tab.badge }}</span>
      </button>
    </div>
    <div class="tabs-body">
      <slot />
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  modelValue: string
  tabs: { id: string; label: string; badge?: number | string }[]
}>()
defineEmits(['update:modelValue'])
</script>

<style scoped>
.ne-tabs { display: flex; flex-direction: column; }
.tabs-header {
  display: flex; border-bottom: 1px solid var(--border-muted); flex-shrink: 0;
}
.tab-btn {
  flex: 1; padding: var(--space-2) var(--space-3); border: none; background: transparent;
  color: var(--text-tertiary); font-size: var(--text-sm); cursor: pointer;
  border-bottom: 2px solid transparent; transition: all var(--transition-fast);
  font-family: inherit; display: flex; align-items: center; justify-content: center; gap: var(--space-1);
}
.tab-btn:hover { color: var(--text-secondary); }
.tab-btn.active { color: var(--text-primary); border-bottom-color: var(--color-primary); }
.tab-badge {
  font-size: 10px; padding: 1px 6px; border-radius: 10px;
  background: var(--bg-panel-secondary); color: var(--text-tertiary);
}
.tabs-body { flex: 1; overflow: hidden; }
</style>
