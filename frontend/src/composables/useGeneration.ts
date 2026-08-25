import { computed } from 'vue'
import { useRoute } from 'vue-router'
import { useGenerationStore } from '@/stores/generation'

export function useGeneration() {
  const route = useRoute()
  const genStore = useGenerationStore()
  const projectId = computed(() => (route.params.id as string) || '')

  const tasks = computed(() => genStore.tasks)
  const currentTask = computed(() => genStore.currentTask)
  const isGenerating = computed(() => genStore.currentTask?.status === 'Running')
  const isCompleted = computed(() => genStore.currentTask?.status === 'Completed')

  function startGeneration(input?: unknown) {
    if (!projectId.value) return
    return genStore.startGeneration(projectId.value, input)
  }

  // 后端 TaskStatus 仅 Pending|Running|Completed|Failed|Cancelled，
  // 旧的三阶段(BuildingContext/Generating/Validating)已在真源删除，简化为 Running→Completed。
  const progressStages = computed(() => {
    if (!genStore.currentTask) return []
    const status = genStore.currentTask.status
    const stages = [
      { id: 'Running', label: '生成中', done: false, active: false },
      { id: 'Completed', label: '完成', done: false, active: false },
    ]
    const order = ['Running', 'Completed']
    const currentIdx = order.indexOf(status)
    return stages.map((s, i) => ({
      ...s,
      done: i < currentIdx || status === 'Completed',
      active: i === currentIdx && status !== 'Completed',
    }))
  })

  return { tasks, currentTask, isGenerating, isCompleted, startGeneration, progressStages }
}
