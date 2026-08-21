import { computed } from 'vue'
import { useRoute } from 'vue-router'
import { useGenerationStore } from '@/stores/generation'
import type { GenerationTaskType } from '@/types'

export function useGeneration() {
  const route = useRoute()
  const genStore = useGenerationStore()
  const projectId = computed(() => (route.params.id as string) || '')

  const tasks = computed(() => genStore.tasks)
  const currentTask = computed(() => genStore.currentTask)
  const isGenerating = computed(() =>
    genStore.currentTask?.status === 'Generating' ||
    genStore.currentTask?.status === 'BuildingContext'
  )
  const isCompleted = computed(() => genStore.currentTask?.status === 'Completed')

  function startGeneration(type: GenerationTaskType, targetId?: string) {
    if (!projectId.value) return
    return genStore.startGeneration(projectId.value, type, targetId)
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
