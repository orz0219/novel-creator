import { defineStore } from "pinia"
import { ref, computed } from "vue"
import type { NarrativeNode, NarrativeNodeType, NarrativeNodeStatus, Storyline, Foreshadowing, TreeNode } from "@/types"
import { narrativeApi, storylineApi, foreshadowApi } from "@/api/story"

export const useStoryStore = defineStore("story", () => {
  const nodes = ref<NarrativeNode[]>([])
  const storylines = ref<Storyline[]>([])
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
    return roots.sort((a, b) => a.sort_order - b.sort_order)
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

  // Fetch storylines
  async function fetchStorylines(projectId: string) {
    try {
      storylines.value = await storylineApi.list(projectId)
    } catch (e: any) {
      error.value = e.message
      storylines.value = []
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

  function selectNode(id: string | null) {
    selectedNodeId.value = id
  }

  const selectedNode = computed(() =>
    nodes.value.find(n => n.id === selectedNodeId.value) || null
  )

  return {
    nodes, storylines, foreshadows, loading, error, selectedNodeId, selectedNode, tree,
    fetchNodes, fetchStorylines, fetchForeshadows,
    createNode, updateNode, deleteNode,
    createStoryline, updateStoryline,
    createForeshadow, updateForeshadow,
    selectNode,
  }
})
