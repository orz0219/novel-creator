import { defineStore } from "pinia"
import { ref } from "vue"
import type { ContextEntity, ContextItem } from "@/types"

export type ContextPolicyType = "Automatic" | "Pinned" | "Excluded"

export const useContextStore = defineStore("context", () => {
  const entities = ref<ContextEntity[]>([])
  const items = ref<ContextItem[]>([])
  const totalTokens = ref(0)
  const loading = ref(false)

  function loadMockContext() {
    entities.value = [
      {
        entity_id: "char-1", entity_name: "林凡", entity_type: "Character",
        relevance: 0.96, policy: "Automatic",
        reasons: ["当前 Scene 主角", "当前剧情线参与者", "最近 3 个 Scene 出现"]
      },
      {
        entity_id: "char-2", entity_name: "苏晚晴", entity_type: "Character",
        relevance: 0.88, policy: "Automatic",
        reasons: ["当前 Scene 参与者", "与主角关系密切"]
      },
      {
        entity_id: "loc-2", entity_name: "地下遗迹", entity_type: "Location",
        relevance: 0.92, policy: "Automatic",
        reasons: ["当前 Scene 地点", "核心剧情地点"]
      },
      {
        entity_id: "fac-1", entity_name: "王家", entity_type: "Faction",
        relevance: 0.75, policy: "Automatic",
        reasons: ["当前剧情线相关势力", "追杀主角"]
      },
      {
        entity_id: "loc-1", entity_name: "黑石城", entity_type: "Location",
        relevance: 0.70, policy: "Automatic",
        reasons: ["当前故事发生的城市", "多个角色所在地"]
      },
      {
        entity_id: "loc-3", entity_name: "古井", entity_type: "Location",
        relevance: 0.65, policy: "Pinned",
        reasons: ["用户手动钉住：重要伏笔地点"]
      },
    ]

    items.value = [
      { id: "ctx-1", type: "timeline", content: "天玄历381年3月12日 14:00", relevance: 0.95, source: "Scene Timeline" },
      { id: "ctx-2", type: "knowledge", content: "林凡知道王家正在追杀自己", relevance: 0.85, source: "Character Knowledge" },
      { id: "ctx-3", type: "constraint", content: "遗迹内可能有残存的远古阵法", relevance: 0.80, source: "World Constraint" },
      { id: "ctx-4", type: "history", content: "上一场景：林凡在古井旁获得黑色令牌", relevance: 0.75, source: "Recent History" },
    ]

    totalTokens.value = 8421
  }

  function togglePin(entityId: string) {
    const entity = entities.value.find(e => e.entity_id === entityId)
    if (entity) {
      entity.policy = entity.policy === "Pinned" ? "Automatic" : "Pinned"
    }
  }

  function toggleExclude(entityId: string) {
    const entity = entities.value.find(e => e.entity_id === entityId)
    if (entity) {
      entity.policy = entity.policy === "Excluded" ? "Automatic" : "Excluded"
    }
  }

  return {
    entities, items, totalTokens, loading,
    loadMockContext, togglePin, toggleExclude,
  }
})
