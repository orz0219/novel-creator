<template>
  <div class="scene-editor">
    <div class="scene-header">
      <div class="scene-info">
        <span class="scene-title">{{ scene?.title || '未选择场景' }}</span>
        <span class="scene-time" v-if="scene?.attributes?.time">{{ scene.attributes.time }}</span>
      </div>
      <div class="scene-actions">
        <button class="action-btn" @click="$emit('save')" :disabled="!isDirty">保存</button>
        <button class="action-btn primary" @click="$emit('generate')">AI 生成</button>
      </div>
    </div>
    <div class="scene-meta" v-if="scene">
      <div class="meta-item" v-if="scene.attributes?.objective">
        <span class="meta-label">目标</span>
        <span class="meta-value">{{ scene.attributes.objective }}</span>
      </div>
      <div class="meta-item" v-if="scene.attributes?.conflict">
        <span class="meta-label">冲突</span>
        <span class="meta-value">{{ scene.attributes.conflict }}</span>
      </div>
    </div>
    <div class="scene-body">
      <StructuredEditor :model-value="content" @update:model-value="$emit('update:content', $event)" />
    </div>
    <div class="scene-footer">
      <span class="footer-stat">字数: {{ wordCount }}</span>
      <span class="footer-stat" :class="{ dirty: isDirty }">{{ isDirty ? '未保存' : '已保存' }}</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import StructuredEditor from './StructuredEditor.vue'
import type { NarrativeNode } from '@/types'

const props = defineProps<{
  scene: NarrativeNode | null
  content: string
  isDirty: boolean
}>()
defineEmits(['save', 'generate', 'update:content'])

const wordCount = computed(() => props.content.replace(/\s/g, '').length)
</script>

<style scoped>
.scene-editor { display: flex; flex-direction: column; height: 100%; }
.scene-header { display: flex; justify-content: space-between; align-items: center; padding: var(--space-3) var(--space-4); border-bottom: 1px solid var(--border-muted); flex-shrink: 0; }
.scene-info { display: flex; align-items: center; gap: var(--space-3); }
.scene-title { font-size: var(--text-md); font-weight: 500; }
.scene-time { font-size: var(--text-xs); color: var(--text-tertiary); }
.scene-actions { display: flex; gap: var(--space-2); }
.action-btn { padding: var(--space-1) var(--space-3); border: 1px solid var(--border-default); background: var(--bg-panel); color: var(--text-secondary); border-radius: var(--radius-sm); font-size: var(--text-xs); cursor: pointer; }
.action-btn:hover:not(:disabled) { background: var(--bg-hover); }
.action-btn:disabled { opacity: 0.5; }
.action-btn.primary { background: var(--color-primary); border-color: var(--color-primary); color: white; }
.action-btn.primary:hover { background: var(--color-primary-hover); }
.scene-meta { padding: var(--space-2) var(--space-4); border-bottom: 1px solid var(--border-muted); background: var(--bg-panel-secondary); flex-shrink: 0; }
.meta-item { display: flex; gap: var(--space-2); font-size: var(--text-xs); padding: 2px 0; }
.meta-label { color: var(--text-tertiary); min-width: 30px; }
.meta-value { color: var(--text-secondary); }
.scene-body { flex: 1; overflow: hidden; min-height: 0; }
.scene-footer { display: flex; gap: var(--space-4); padding: var(--space-2) var(--space-4); border-top: 1px solid var(--border-muted); font-size: var(--text-xs); color: var(--text-tertiary); flex-shrink: 0; }
.footer-stat.dirty { color: var(--color-warning); }
</style>
