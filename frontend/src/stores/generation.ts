import { defineStore } from "pinia"
import { ref } from "vue"
import type { GenerationTask } from "@/types"
import { generationApi } from "@/api/generation"

export const useGenerationStore = defineStore("generation", () => {
  const tasks = ref<GenerationTask[]>([])
  const currentTask = ref<GenerationTask | null>(null)
  const loading = ref(false)
  const sseConnected = ref(false)
  let pollTimer: ReturnType<typeof setInterval> | null = null

  // Load real generation tasks for a project (replaces the old mock loader).
  async function loadTasks(projectId: string) {
    loading.value = true
    try {
      tasks.value = await generationApi.list(projectId)
    } catch {
      tasks.value = []
    } finally {
      loading.value = false
    }
  }

  function stopPolling() {
    if (pollTimer) {
      clearInterval(pollTimer)
      pollTimer = null
    }
  }

  // Start a real generation and poll its status until it reaches a terminal state.
  async function startGeneration(projectId: string, type: string, targetId?: string) {
    stopPolling()
    const task = await generationApi.start(projectId, { type, target_id: targetId })
    currentTask.value = task
    pollTimer = setInterval(async () => {
      try {
        const t = await generationApi.get(task.id)
        currentTask.value = t
        if (t.status === "Completed" || t.status === "Failed" || t.status === "Cancelled") {
          stopPolling()
          loadTasks(projectId)
        }
      } catch {
        stopPolling()
      }
    }, 2500)
    // 创建任务后必须触发后端真正执行，否则任务会永远卡在 Pending。
    generationApi.execute(task.id).catch((e) => {
      console.error("generation execute failed", e)
      stopPolling()
    })
    return task
  }

  async function cancelGeneration(id: string) {
    await generationApi.cancel(id)
    if (currentTask.value?.id === id) {
      currentTask.value = { ...currentTask.value, status: "Cancelled" }
    }
  }

  return {
    tasks,
    currentTask,
    loading,
    sseConnected,
    loadTasks,
    startGeneration,
    cancelGeneration,
  }
})
