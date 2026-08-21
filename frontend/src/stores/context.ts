import { defineStore } from "pinia"
import { ref } from "vue"
import { contextApi } from "@/api/context"
import type { ContextEntity, ContextItem } from "@/types"

export type ContextPolicyType = "Automatic" | "Pinned" | "Excluded"

export const useContextStore = defineStore("context", () => {
  const entities = ref<ContextEntity[]>([])
  const items = ref<ContextItem[]>([])
  const totalTokens = ref(0)
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function loadContext(sceneId: string) {
    if (!sceneId) {
      reset()
      return
    }
    loading.value = true
    error.value = null
    try {
      const snap = await contextApi.getSceneContext(sceneId)
      entities.value = snap.entities ?? []
      items.value = snap.items ?? []
      totalTokens.value = snap.total_tokens ?? 0
    } catch (e) {
      reset()
      error.value = e instanceof Error ? e.message : "加载上下文失败"
    } finally {
      loading.value = false
    }
  }

  function reset() {
    entities.value = []
    items.value = []
    totalTokens.value = 0
    error.value = null
  }

  function togglePin(entityId: string) {
    const entity = entities.value.find((e) => e.entity_id === entityId)
    if (entity) {
      entity.policy = entity.policy === "Pinned" ? "Automatic" : "Pinned"
    }
  }

  function toggleExclude(entityId: string) {
    const entity = entities.value.find((e) => e.entity_id === entityId)
    if (entity) {
      entity.policy = entity.policy === "Excluded" ? "Automatic" : "Excluded"
    }
  }

  return {
    entities,
    items,
    totalTokens,
    loading,
    error,
    loadContext,
    reset,
    togglePin,
    toggleExclude,
  }
})
