<template>
  <div class="chapter-editor">
    <div class="chapter-header">
      <span class="chapter-title">{{ chapter?.title || '未选择章节' }}</span>
      <span class="chapter-status" v-if="chapter">{{ chapter.status }}</span>
    </div>
    <div class="chapter-scenes">
      <div v-for="scene in scenes" :key="scene.id" class="scene-item" :class="{ active: activeSceneId === scene.id }" @click="$emit('select-scene', scene.id)">
        <span class="scene-dot" :class="scene.status"></span>
        <span class="scene-name">{{ scene.title }}</span>
        <span class="scene-words">{{ scene.wordCount || 0 }} 字</span>
      </div>
    </div>
    <div class="chapter-footer">
      <button class="add-scene-btn" @click="$emit('add-scene')">+ 添加场景</button>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { NarrativeNode } from '@/types'
defineProps<{
  chapter: NarrativeNode | null
  scenes: (NarrativeNode & { wordCount?: number })[]
  activeSceneId: string | null
}>()
defineEmits(['select-scene', 'add-scene'])
</script>

<style scoped>
.chapter-editor { display: flex; flex-direction: column; }
.chapter-header { display: flex; justify-content: space-between; align-items: center; padding: var(--space-3) var(--space-4); border-bottom: 1px solid var(--border-muted); }
.chapter-title { font-size: var(--text-md); font-weight: 500; }
.chapter-status { font-size: var(--text-xs); color: var(--text-tertiary); }
.chapter-scenes { flex: 1; overflow-y: auto; padding: var(--space-2); }
.scene-item { display: flex; align-items: center; gap: var(--space-2); padding: var(--space-2) var(--space-3); border-radius: var(--radius-sm); cursor: pointer; transition: background var(--transition-fast); }
.scene-item:hover { background: var(--bg-hover); }
.scene-item.active { background: var(--color-primary-subtle); }
.scene-dot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; }
.scene-dot.Completed { background: var(--color-success); }
.scene-dot.InProgress { background: var(--color-accent); }
.scene-dot.Planned { background: var(--text-tertiary); }
.scene-dot.Draft { background: var(--color-warning); }
.scene-name { flex: 1; font-size: var(--text-sm); }
.scene-words { font-size: var(--text-xs); color: var(--text-tertiary); }
.chapter-footer { padding: var(--space-2); border-top: 1px solid var(--border-muted); }
.add-scene-btn { width: 100%; padding: var(--space-2); border: 1px dashed var(--border-default); background: transparent; color: var(--text-tertiary); border-radius: var(--radius-sm); font-size: var(--text-xs); cursor: pointer; }
.add-scene-btn:hover { border-color: var(--border-emphasis); color: var(--text-primary); background: var(--bg-hover); }
</style>
