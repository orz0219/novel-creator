import { defineStore } from "pinia"
import { ref, computed } from "vue"
import type { NarrativeNode, NarrativeNodeType, NarrativeNodeStatus, Storyline, StorylineRelation, Foreshadowing, TreeNode } from "@/types"
import { narrativeApi, storylineApi, foreshadowApi } from "@/api/story"

export const useStoryStore = defineStore("story", () => {
  const nodes = ref<NarrativeNode[]>([])
  const storylines = ref<Storyline[]>([])
  const storylineRelations = ref<StorylineRelation[]>([])  // parent → child 挂载关系
  const foreshadows = ref<Foreshadowing[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)
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
    // 显式按 sort_order 排每一层：接口返回的是全项目 ORDER BY sort_order 的扁平列表，
    // 只排 roots 的话，children 的顺序就依赖「组内相对顺序恰好被保持」这个巧合。
    const bySortOrder = (a: TreeNode, b: TreeNode) => a.sort_order - b.sort_order
    const sortLevels = (list: TreeNode[]) => {
      list.sort(bySortOrder)
      for (const item of list) sortLevels(item.children)
    }
    sortLevels(roots)
    return roots
  })

  // Fetch narrative nodes
  async function fetchNodes(projectId: string) {
    loading.value = true
    error.value = null
    try {
      nodes.value = await narrativeApi.listNodes(projectId)
    } catch (e: any) {
      error.value = e.message
      nodes.value = []
    } finally {
      loading.value = false
    }
  }

  // Fetch storylines + 挂载关系
  async function fetchStorylines(projectId: string) {
    try {
      // 并行拉两条：storyline 列表 + 关系列表
      const [sList, rels] = await Promise.all([
        storylineApi.list(projectId),
        storylineApi.listRelations(projectId).catch(() => []),
      ])
      storylines.value = sList
      storylineRelations.value = rels
    } catch (e: any) {
      error.value = e.message
      storylines.value = []
      storylineRelations.value = []
    }
  }

  // Fetch foreshadows
  async function fetchForeshadows(projectId: string) {
    try {
      foreshadows.value = await foreshadowApi.list(projectId)
    } catch (e: any) {
      error.value = e.message
      foreshadows.value = []
    }
  }

  // CRUD: Narrative nodes
  async function createNode(projectId: string, data: Partial<NarrativeNode>) {
    const result = await narrativeApi.createNode(projectId, data)
    nodes.value.push(result)
    return result
  }

  async function updateNode(id: string, data: Partial<NarrativeNode>) {
    const result = await narrativeApi.updateNode(id, data)
    const idx = nodes.value.findIndex(n => n.id === id)
    if (idx !== -1) nodes.value[idx] = result
    return result
  }

  /**
   * 只改节点状态（看板拖拽/快捷按钮用）。
   * 后端 PUT /narrative/{id} 是部分更新，只传 status 不会碰 title/content。
   * 乐观更新：先动界面，请求失败必须把本地状态退回原值再抛错 ——
   * 退回是为了让界面与后端保持一致，抛错是为了让调用方提示用户，两者都不能省。
   */
  async function setNodeStatus(id: string, status: NarrativeNodeStatus) {
    const idx = nodes.value.findIndex(n => n.id === id)
    if (idx === -1) throw new Error(`节点不在当前列表中，无法改状态：${id}`)
    const previous = nodes.value[idx]
    if (previous.status === status) return previous

    nodes.value[idx] = { ...previous, status }
    try {
      const result = await narrativeApi.updateNode(id, { status })
      nodes.value[idx] = result
      return result
    } catch (e) {
      nodes.value[idx] = previous
      throw e
    }
  }

  async function deleteNode(id: string) {
    await narrativeApi.deleteNode(id)
    nodes.value = nodes.value.filter(n => n.id !== id)
  }

  // CRUD: Storylines
  async function createStoryline(projectId: string, data: Partial<Storyline>) {
    const result = await storylineApi.create(projectId, data)
    storylines.value.push(result)
    return result
  }

  async function updateStoryline(id: string, data: Partial<Storyline>) {
    const result = await storylineApi.update(id, data)
    const idx = storylines.value.findIndex(s => s.id === id)
    if (idx !== -1) storylines.value[idx] = result
    return result
  }

  async function deleteStoryline(id: string) {
    await storylineApi.delete(id)
    storylines.value = storylines.value.filter(s => s.id !== id)
  }

  // CRUD: Foreshadows
  async function createForeshadow(projectId: string, data: Partial<Foreshadowing>) {
    const result = await foreshadowApi.create(projectId, data)
    foreshadows.value.push(result)
    return result
  }

  async function updateForeshadow(id: string, data: Partial<Foreshadowing>) {
    const result = await foreshadowApi.update(id, data)
    const idx = foreshadows.value.findIndex(f => f.id === id)
    if (idx !== -1) foreshadows.value[idx] = result
    return result
  }

  async function deleteForeshadow(id: string) {
    await foreshadowApi.delete(id)
    foreshadows.value = foreshadows.value.filter(f => f.id !== id)
  }

  function selectNode(id: string | null) {
    selectedNodeId.value = id
  }

  const selectedNode = computed(() =>
    nodes.value.find(n => n.id === selectedNodeId.value) || null
  )

  return {
    nodes, storylines, storylineRelations, foreshadows, loading, error, selectedNodeId, selectedNode, tree,
    fetchNodes, fetchStorylines, fetchForeshadows,
    createNode, updateNode, deleteNode, setNodeStatus,
    createStoryline, updateStoryline, deleteStoryline,
    createForeshadow, updateForeshadow, deleteForeshadow,
    selectNode,
  }
})
