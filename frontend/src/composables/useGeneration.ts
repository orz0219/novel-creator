import { computed } from 'vue'
import { useGenerationStore } from '@/stores/generation'
import type { GenerationTaskType } from '@/types'

export function useGeneration() {
  const genStore = useGenerationStore()

  const tasks = computed(() => genStore.tasks)
  const currentTask = computed(() => genStore.currentTask)
  const isGenerating = computed(() =>
    genStore.currentTask?.status === 'Generating' ||
    genStore.currentTask?.status === 'BuildingContext'
  )
  const isCompleted = computed(() => genStore.currentTask?.status === 'Completed')

  function startGeneration(type: GenerationTaskType, targetId?: string) {
    return genStore.startGeneration(type, targetId)
  }

  const progressStages = computed(() => {
    if (!genStore.currentTask) return []
    const status = genStore.currentTask.status
    const stages = [
      { id: 'BuildingContext', label: '构建上下文', done: false, active: false },
      { id: 'Generating', label: '生成内容', done: false, active: false },
      { id: 'Validating', label: '验证', done: false, active: false },
      { id: 'Completed', label: '完成', done: false, active: false },
    ]
    const order = ['BuildingContext', 'Generating', 'Validating', 'Completed']
    const currentIdx = order.indexOf(status)
    return stages.map((s, i) => ({
      ...s,
      done: i < currentIdx,
      active: i === currentIdx,
    }))
  })

  return { tasks, currentTask, isGenerating, isCompleted, startGeneration, progressStages }
}
