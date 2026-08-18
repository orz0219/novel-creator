import { defineStore } from "pinia"
import { ref, computed } from "vue"
import type { NarrativeNode, NarrativeNodeType, NarrativeNodeStatus, Storyline, Foreshadowing, TreeNode } from "@/types"

export const useStoryStore = defineStore("story", () => {
  const nodes = ref<NarrativeNode[]>([])
  const storylines = ref<Storyline[]>([])
  const foreshadows = ref<Foreshadowing[]>([])
  const loading = ref(false)
  const selectedNodeId = ref<string | null>(null)

  const tree = computed<TreeNode[]>(() => {
    const nodeMap = new Map<string, TreeNode>()
    const roots: TreeNode[] = []
    for (const node of nodes.value) {
      nodeMap.set(node.id, { ...node, children: [] })
    }
    for (const node of nodes.value) {
      const treeNode = nodeMap.get(node.id)!
      if (node.parent_id && nodeMap.has(node.parent_id)) {
        nodeMap.get(node.parent_id)!.children.push(treeNode)
      } else {
        roots.push(treeNode)
      }
    }
    return roots.sort((a, b) => a.sort_order - b.sort_order)
  })

  function loadMockData() {
    nodes.value = [
      { id: "vol-1", project_id: "p1", world_id: "w1", node_type: "Volume" as NarrativeNodeType,
        title: "第一卷：黑石城", description: "主角在黑石城的冒险开始",
        attributes: { mission: "让主角离开黑石城", theme: "成长与觉醒" },
        sort_order: 1, status: "InProgress" as NarrativeNodeStatus,
        created_at: "2024-01-15T08:00:00Z", updated_at: "2024-03-12T14:00:00Z" },
      { id: "arc-1", project_id: "p1", world_id: "w1", node_type: "Arc" as NarrativeNodeType,
        parent_id: "vol-1", title: "初入黑石城", description: "林凡初到黑石城，结识苏晚晴",
        attributes: { goal: "建立主角在黑石城的基础" },
        sort_order: 1, status: "Completed" as NarrativeNodeStatus,
        created_at: "2024-01-15T08:00:00Z", updated_at: "2024-02-20T10:00:00Z" },
      { id: "arc-2", project_id: "p1", world_id: "w1", node_type: "Arc" as NarrativeNodeType,
        parent_id: "vol-1", title: "王家追杀", description: "王家发现林凡持有古玉，开始追杀",
        attributes: { goal: "制造核心冲突", conflict: "生存与真相" },
        sort_order: 2, status: "InProgress" as NarrativeNodeStatus,
        created_at: "2024-02-01T08:00:00Z", updated_at: "2024-03-12T14:00:00Z" },
      { id: "ch-1", project_id: "p1", world_id: "w1", node_type: "Chapter" as NarrativeNodeType,
        parent_id: "arc-1", title: "第一章：边境来客", description: "林凡抵达黑石城",
        attributes: {}, sort_order: 1, status: "Completed" as NarrativeNodeStatus,
        created_at: "2024-01-15T08:00:00Z", updated_at: "2024-01-20T10:00:00Z" },
      { id: "ch-2", project_id: "p1", world_id: "w1", node_type: "Chapter" as NarrativeNodeType,
        parent_id: "arc-1", title: "第二章：黑市风云", description: "林凡进入黑市",
        attributes: {}, sort_order: 2, status: "Completed" as NarrativeNodeStatus,
        created_at: "2024-01-20T10:00:00Z", updated_at: "2024-02-05T14:00:00Z" },
      { id: "ch-3", project_id: "p1", world_id: "w1", node_type: "Chapter" as NarrativeNodeType,
        parent_id: "arc-2", title: "第三章：暗流涌动", description: "王家开始注意到林凡",
        attributes: {}, sort_order: 3, status: "Completed" as NarrativeNodeStatus,
        created_at: "2024-02-05T10:00:00Z", updated_at: "2024-02-15T16:00:00Z" },
      { id: "ch-4", project_id: "p1", world_id: "w1", node_type: "Chapter" as NarrativeNodeType,
        parent_id: "arc-2", title: "第四章：地下遗迹", description: "林凡发现地下遗迹",
        attributes: {}, sort_order: 4, status: "InProgress" as NarrativeNodeStatus,
        created_at: "2024-02-15T10:00:00Z", updated_at: "2024-03-12T14:00:00Z" },
      { id: "ch-5", project_id: "p1", world_id: "w1", node_type: "Chapter" as NarrativeNodeType,
        parent_id: "arc-2", title: "第五章：逃离黑石城", description: "林凡被迫离开黑石城",
        attributes: {}, sort_order: 5, status: "Planned" as NarrativeNodeStatus,
        created_at: "2024-03-01T10:00:00Z", updated_at: "2024-03-01T10:00:00Z" },
      { id: "scene-1", project_id: "p1", world_id: "w1", node_type: "Scene" as NarrativeNodeType,
        parent_id: "ch-4", title: "场景1：遗迹入口", description: "林凡找到地下遗迹的入口",
        attributes: {
          objective: "发现遗迹入口", pov_character_id: "char-1", location_id: "loc-2",
          time: "天玄历381年3月12日 14:00", characters_present: ["char-1", "char-2"],
        },
        sort_order: 1, status: "InProgress" as NarrativeNodeStatus,
        created_at: "2024-03-10T10:00:00Z", updated_at: "2024-03-12T14:00:00Z" },
      { id: "scene-2", project_id: "p1", world_id: "w1", node_type: "Scene" as NarrativeNodeType,
        parent_id: "ch-4", title: "场景2：机关重重", description: "林凡和苏晚晴在遗迹中遭遇机关",
        attributes: {
          objective: "通过遗迹机关", pov_character_id: "char-1", location_id: "loc-2",
          time: "天玄历381年3月12日 16:00", characters_present: ["char-1", "char-2"],
        },
        sort_order: 2, status: "Draft" as NarrativeNodeStatus,
        created_at: "2024-03-12T10:00:00Z", updated_at: "2024-03-12T10:00:00Z" },
    ]

    storylines.value = [
      { id: "sl-1", project_id: "p1", name: "主角成长线", description: "林凡从散修到强者的成长之路",
        status: "Active", importance: "Main",
        created_at: "2024-01-15T08:00:00Z", updated_at: "2024-03-12T14:00:00Z" },
      { id: "sl-2", project_id: "p1", name: "王家追杀线", description: "王家追杀林凡的阴谋",
        status: "Active", importance: "Main",
        created_at: "2024-02-01T08:00:00Z", updated_at: "2024-03-12T14:00:00Z" },
      { id: "sl-3", project_id: "p1", name: "古井秘密线", description: "古井背后的远古秘密",
        status: "Planned", importance: "Important",
        created_at: "2024-02-20T10:00:00Z", updated_at: "2024-02-20T10:00:00Z" },
    ]

    foreshadows.value = [
      { id: "fs-1", project_id: "p1", name: "黑色令牌",
        description: "林凡在古井旁获得的神秘令牌，暗示着远古势力",
        status: "Introduced", importance: "Core", hint_level: "Subtle",
        planted_scene_id: "scene-1", related_entity_ids: ["char-1", "loc-3"],
        created_at: "2024-02-20T14:00:00Z", updated_at: "2024-03-12T14:00:00Z" },
      { id: "fs-2", project_id: "p1", name: "苏晚晴的身份",
        description: "苏晚晴似乎知道很多不应该知道的事情",
        status: "Active", importance: "Important", hint_level: "Direct",
        related_entity_ids: ["char-2"],
        created_at: "2024-01-20T10:00:00Z", updated_at: "2024-03-10T16:00:00Z" },
    ]
  }

  function selectNode(id: string | null) {
    selectedNodeId.value = id
  }

  const selectedNode = computed(() =>
    nodes.value.find(n => n.id === selectedNodeId.value) || null
  )

  return {
    nodes, storylines, foreshadows, loading, selectedNodeId, selectedNode, tree,
    loadMockData, selectNode,
  }
})