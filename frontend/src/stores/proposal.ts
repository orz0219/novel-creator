import { defineStore } from "pinia"
import { ref } from "vue"
import type { Proposal } from "@/types"

export const useProposalStore = defineStore("proposal", () => {
  const proposals = ref<Proposal[]>([])
  const currentProposal = ref<Proposal | null>(null)
  const loading = ref(false)

  function loadMockData() {
    proposals.value = [
      {
        id: "prop-1", generation_task_id: "gen-1", status: "Approved" as any,
        changes: [
          { id: "pc-1", change_type: "Added" as any, target_entity_type: "Location",
            target_entity_name: "地下遗迹", description: "新增地点：黑石城下方的远古遗迹",
            risk_level: "Low" as any, accepted: true },
          { id: "pc-2", change_type: "Added" as any, target_entity_type: "Location",
            target_entity_name: "炼器坊", description: "新增遗迹内区域：远古炼器坊",
            risk_level: "Low" as any, accepted: true },
          { id: "pc-3", change_type: "Modified" as any, target_entity_type: "Location",
            target_entity_id: "loc-1", target_entity_name: "黑石城",
            description: "更新黑石城地下结构描述", risk_level: "Low" as any, accepted: true },
        ],
        validation_results: [
          { id: "vr-1", severity: "Info" as any, dimension: "World", message: "新增地点与现有世界观一致", related_entity_ids: [] },
        ],
        reason: "为第三卷提供核心探索场景",
        created_at: "2024-02-01T12:05:00Z", reviewed_at: "2024-02-01T14:00:00Z"
      },
      {
        id: "prop-2", generation_task_id: "gen-2", status: "Approved" as any,
        changes: [
          { id: "pc-4", change_type: "Added" as any, target_entity_type: "Location",
            target_entity_name: "古井", description: "新增地点：黑市深处的神秘古井",
            risk_level: "Low" as any, accepted: true },
          { id: "pc-5", change_type: "Modified" as any, target_entity_type: "Character",
            target_entity_id: "char-1", target_entity_name: "林凡",
            description: "林凡获得黑色令牌", risk_level: "Medium" as any, accepted: true },
        ],
        validation_results: [
          { id: "vr-2", severity: "Warning" as any, dimension: "Timeline",
            message: "古井的引入时间需要与黑市建立时间一致", related_entity_ids: ["loc-3"] },
        ],
        reason: "引入重要伏笔元素",
        created_at: "2024-02-20T14:05:00Z", reviewed_at: "2024-02-20T16:00:00Z"
      },
    ]
  }

  return {
    proposals, currentProposal, loading,
    loadMockData,
  }
})
