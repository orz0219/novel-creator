<template>
  <div class="storylines-page">
    <div class="page-header">
      <h1 class="page-title">剧情线</h1>
      <button class="btn-primary">+ 新建剧情线</button>
    </div>
    <div class="storyline-list">
      <div v-for="sl in storyStore.storylines" :key="sl.id" class="storyline-card">
        <div class="sl-header">
          <span class="sl-dot" :class="sl.status.toLowerCase()"></span>
          <span class="sl-name">{{ sl.name }}</span>
          <span class="sl-importance" :class="sl.importance.toLowerCase()">{{ sl.importance }}</span>
          <StatusBadge :status="sl.status.toLowerCase()" :label="sl.status" />
        </div>
        <div class="sl-desc">{{ sl.description }}</div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useStoryStore } from '@/stores/story'
import StatusBadge from '@/components/ui/StatusBadge.vue'
const storyStore = useStoryStore()
storyStore.loadMockData()
</script>

<style scoped>
.storylines-page { height: 100%; overflow-y: auto; padding: var(--space-6) var(--space-8); }
.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--space-6); }
.page-title { font-size: var(--text-2xl); font-weight: 700; font-family: var(--font-serif); }
.btn-primary { padding: var(--space-2) var(--space-4); background: var(--color-primary); border: none; color: white; border-radius: var(--radius-sm); font-size: var(--text-sm); cursor: pointer; }
.storyline-list { display: flex; flex-direction: column; gap: var(--space-3); }
.storyline-card { padding: var(--space-4) var(--space-5); border: 1px solid var(--border-default); border-radius: var(--radius-md); background: var(--bg-panel); }
.sl-header { display: flex; align-items: center; gap: var(--space-3); margin-bottom: var(--space-2); }
.sl-dot { width: 8px; height: 8px; border-radius: 50%; }
.sl-dot.active { background: var(--color-success); }
.sl-dot.planned { background: var(--text-tertiary); }
.sl-name { font-size: var(--text-md); font-weight: 600; }
.sl-importance { font-size: var(--text-xs); padding: 2px 8px; border-radius: 10px; }
.sl-importance.main { background: var(--color-primary-subtle); color: var(--color-primary-text); }
.sl-importance.important { background: var(--color-accent-subtle); color: var(--color-accent); }
.sl-desc { font-size: var(--text-sm); color: var(--text-secondary); }
</style>
