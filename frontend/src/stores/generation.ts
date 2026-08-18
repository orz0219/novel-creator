import { defineStore } from "pinia"
import { ref } from "vue"
import type { GenerationTask, GenerationTaskStatus } from "@/types"

export const useGenerationStore = defineStore("generation", () => {
  const tasks = ref<GenerationTask[]>([])
  const currentTask = ref<GenerationTask | null>(null)
  const loading = ref(false)
  const sseConnected = ref(false)

  function loadMockData() {
    tasks.value = [
      {
        id: "gen-1", type: "GenerateLocation" as any, target_id: "loc-2",
        model: "mimo-v2.5", parameters: { description: "为第三卷设计地下遗迹" },
        status: "Completed" as any, context_tokens: 6200,
        result: "已生成地下遗迹设定，包含3个区域和5个机关。",
        created_at: "2024-02-01T12:00:00Z", updated_at: "2024-02-01T12:05:00Z"
      },
      {
        id: "gen-2", type: "GenerateScene" as any, target_id: "scene-1",
        model: "mimo-v2.5", parameters: { word_count: 2000, style: "紧凑紧张" },
        status: "Completed" as any, context_tokens: 8421,
        result: "场景已生成，约2100字。",
        created_at: "2024-03-10T10:00:00Z", updated_at: "2024-03-10T10:08:00Z"
      },
      {
        id: "gen-3", type: "GenerateScene" as any, target_id: "scene-2",
        model: "mimo-v2.5", parameters: { word_count: 2500, style: "紧张刺激" },
        status: "BuildingContext" as any, context_tokens: 0,
        created_at: "2024-03-12T14:00:00Z", updated_at: "2024-03-12T14:00:00Z"
      },
    ]
  }

  function startGeneration(type: string, targetId?: string) {
    const task: GenerationTask = {
      id: "gen-" + Date.now(),
      type: type as any,
      target_id: targetId,
      model: "mimo-v2.5",
      parameters: {},
      status: "Pending" as any,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    }
    tasks.value.unshift(task)
    currentTask.value = task
    simulateProgress(task)
    return task
  }

  function simulateProgress(task: GenerationTask) {
    const stages: GenerationTaskStatus[] = ["BuildingContext", "Generating", "Validating", "Completed"]
    let stageIndex = 0
    const interval = setInterval(() => {
      if (stageIndex < stages.length) {
        task.status = stages[stageIndex]
        task.updated_at = new Date().toISOString()
        if (stageIndex === 0) task.context_tokens = 8421
        if (stageIndex === 3) task.result = "生成完成。"
        stageIndex++
      } else {
        clearInterval(interval)
      }
    }, 2000)
  }

  return {
    tasks, currentTask, loading, sseConnected,
    loadMockData, startGeneration,
  }
})
