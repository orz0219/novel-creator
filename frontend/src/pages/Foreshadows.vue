<template>
  <div class="foreshadows-page">
    <div class="page-header">
      <h1 class="page-title">伏笔</h1>
      <button class="btn-primary">+ 新建伏笔</button>
    </div>
    <div class="foreshadow-list">
      <div v-for="fs in storyStore.foreshadows" :key="fs.id" class="foreshadow-card">
        <div class="fs-header">
          <span class="fs-badge" :class="fs.status.toLowerCase()">{{ fs.status }}</span>
          <span class="fs-name">{{ fs.name }}</span>
          <span class="fs-importance">{{ fs.importance }}</span>
          <span class="fs-hint">暗示级别：{{ fs.hint_level }}</span>
        </div>
        <div class="fs-desc">{{ fs.description }}</div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useStoryStore } from '@/stores/story'
const storyStore = useStoryStore()
storyStore.loadMockData()
</script>

<style scoped>
.foreshadows-page { height: 100%; overflow-y: auto; padding: var(--space-6) var(--space-8); }
.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--space-6); }
.page-title { font-size: var(--text-2xl); font-weight: 700; font-family: var(--font-serif); }
.btn-primary { padding: var(--space-2) var(--space-4); background: var(--color-primary); border: none; color: white; border-radius: var(--radius-sm); font-size: var(--text-sm); cursor: pointer; }
.foreshadow-list { display: flex; flex-direction: column; gap: var(--space-3); }
.foreshadow-card { padding: var(--space-4) var(--space-5); border: 1px solid var(--border-default); border-radius: var(--radius-md); background: var(--bg-panel); }
.fs-header { display: flex; align-items: center; gap: var(--space-3); margin-bottom: var(--space-2); }
.fs-badge { font-size: 10px; padding: 2px 8px; border-radius: 3px; }
.fs-badge.introduced { background: var(--color-info-subtle); color: var(--color-info); }
.fs-badge.active { background: var(--color-warning-subtle); color: var(--color-warning); }
.fs-badge.planned { background: var(--bg-panel-secondary); color: var(--text-tertiary); }
.fs-name { font-size: var(--text-md); font-weight: 600; }
.fs-importance { font-size: var(--text-xs); color: var(--text-tertiary); }
.fs-hint { font-size: var(--text-xs); color: var(--text-tertiary); margin-left: auto; }
.fs-desc { font-size: var(--text-sm); color: var(--text-secondary); }
</style>
