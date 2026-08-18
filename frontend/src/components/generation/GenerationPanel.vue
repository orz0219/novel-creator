<template>
  <div class="generation-panel">
    <div class="panel-header">
      <span class="panel-title">AI 生成</span>
      <button class="start-btn" @click="$emit('start')" :disabled="isGenerating">
        {{ isGenerating ? '生成中...' : '开始生成' }}
      </button>
    </div>
    <div class="panel-body">
      <div class="task-info" v-if="currentTask">
        <div class="task-type">{{ currentTask.type }}</div>
        <GenerationProgress :status="currentTask.status" />
        <div class="task-result" v-if="currentTask.result">
          <div class="result-label">结果</div>
          <div class="result-text">{{ currentTask.result }}</div>
        </div>
      </div>
      <div class="empty-state" v-else>选择场景后点击开始生成</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import GenerationProgress from './GenerationProgress.vue'
import type { GenerationTask } from '@/types'
defineProps<{
  currentTask: GenerationTask | null
  isGenerating: boolean
}>()
defineEmits(['start'])
</script>

<style scoped>
.generation-panel { display: flex; flex-direction: column; }
.panel-header { display: flex; justify-content: space-between; align-items: center; padding: var(--space-3) var(--space-4); border-bottom: 1px solid var(--border-muted); }
.panel-title { font-size: var(--text-sm); font-weight: 600; }
.start-btn { padding: var(--space-1) var(--space-3); background: var(--color-primary); border: none; color: white; border-radius: var(--radius-sm); font-size: var(--text-xs); cursor: pointer; }
.start-btn:disabled { opacity: 0.5; cursor: not-allowed; }
.start-btn:hover:not(:disabled) { background: var(--color-primary-hover); }
.panel-body { padding: var(--space-3); }
.task-type { font-size: var(--text-sm); font-weight: 500; margin-bottom: var(--space-2); }
.task-result { margin-top: var(--space-3); padding-top: var(--space-3); border-top: 1px solid var(--border-muted); }
.result-label { font-size: var(--text-xs); color: var(--text-tertiary); margin-bottom: var(--space-1); }
.result-text { font-size: var(--text-sm); color: var(--text-secondary); }
.empty-state { padding: var(--space-4); text-align: center; color: var(--text-tertiary); font-size: var(--text-sm); }
</style>
