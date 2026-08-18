<template>
  <div class="editor-toolbar">
    <div class="toolbar-group">
      <button class="toolbar-btn" @click="$emit('save')" :disabled="!isDirty" title="保存">
        <span>💾</span>
      </button>
      <button class="toolbar-btn" @click="$emit('generate')" title="AI 生成">
        <span>🤖</span>
      </button>
    </div>
    <div class="toolbar-separator"></div>
    <div class="toolbar-group">
      <button class="toolbar-btn" @click="$emit('command', 'undo')" title="撤销">↩️</button>
      <button class="toolbar-btn" @click="$emit('command', 'redo')" title="重做">↪️</button>
    </div>
    <div class="toolbar-separator"></div>
    <div class="toolbar-group">
      <span class="toolbar-info">字数: {{ wordCount }}</span>
      <span class="toolbar-info" :class="{ dirty: isDirty }">{{ isDirty ? '未保存' : '已保存' }}</span>
    </div>
    <div class="toolbar-right">
      <slot name="right" />
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  wordCount: number
  isDirty: boolean
}>()
defineEmits(['save', 'generate', 'command'])
</script>

<style scoped>
.editor-toolbar {
  display: flex; align-items: center; gap: var(--space-2);
  padding: var(--space-2) var(--space-4); border-bottom: 1px solid var(--border-muted);
  background: var(--bg-panel); flex-shrink: 0;
}
.toolbar-group { display: flex; align-items: center; gap: var(--space-1); }
.toolbar-btn {
  display: flex; align-items: center; justify-content: center;
  width: 28px; height: 28px; border: none; background: transparent;
  color: var(--text-secondary); border-radius: var(--radius-sm); cursor: pointer;
  transition: all var(--transition-fast); font-size: var(--text-sm);
}
.toolbar-btn:hover:not(:disabled) { background: var(--bg-hover); color: var(--text-primary); }
.toolbar-btn:disabled { opacity: 0.4; cursor: not-allowed; }
.toolbar-separator { width: 1px; height: 16px; background: var(--border-muted); }
.toolbar-info { font-size: var(--text-xs); color: var(--text-tertiary); padding: 0 var(--space-2); }
.toolbar-info.dirty { color: var(--color-warning); }
.toolbar-right { margin-left: auto; display: flex; align-items: center; gap: var(--space-2); }
</style>
