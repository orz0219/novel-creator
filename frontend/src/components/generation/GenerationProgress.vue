<template>
  <div class="generation-progress">
    <div v-for="stage in stages" :key="stage.id" class="progress-step" :class="{ done: isDone(stage.id), active: status === stage.id }">
      <span class="step-dot"></span>
      <span class="step-label">{{ stage.label }}</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
const props = defineProps<{ status: string }>()
const stages = [
  { id: 'BuildingContext', label: '构建上下文' },
  { id: 'Generating', label: '生成内容' },
  { id: 'Validating', label: '验证' },
  { id: 'Completed', label: '完成' },
]
function isDone(stageId: string): boolean {
  const order = ['BuildingContext', 'Generating', 'Validating', 'Completed']
  return order.indexOf(props.status) > order.indexOf(stageId)
}
</script>

<style scoped>
.generation-progress { display: flex; flex-direction: column; gap: var(--space-2); }
.progress-step { display: flex; align-items: center; gap: var(--space-2); font-size: var(--text-xs); color: var(--text-tertiary); }
.progress-step.done { color: var(--color-success); }
.progress-step.active { color: var(--text-primary); }
.step-dot { width: 8px; height: 8px; border-radius: 50%; border: 1.5px solid currentColor; flex-shrink: 0; }
.progress-step.done .step-dot { background: var(--color-success); border-color: var(--color-success); }
.progress-step.active .step-dot { border-color: var(--color-accent); animation: pulse 1.5s infinite; }
@keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.4; } }
</style>
