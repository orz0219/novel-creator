import { describe, it, expect, vi, beforeEach } from "vitest"
import { createPinia, setActivePinia } from "pinia"

vi.mock("@/api/story", () => {
  return {
    narrativeApi: {
      listNodes: vi.fn(),
      createNode: vi.fn(),
      updateNode: vi.fn(),
      deleteNode: vi.fn(),
    },
    storylineApi: {
      list: vi.fn(),
      create: vi.fn(),
      update: vi.fn(),
      delete: vi.fn(),
    },
    foreshadowApi: {
      list: vi.fn(),
      create: vi.fn(),
      update: vi.fn(),
      delete: vi.fn(),
    },
  }
})

import { narrativeApi, storylineApi, foreshadowApi } from "@/api/story"
import { useStoryStore } from "@/stores/story"
import type { NarrativeNode, Storyline, Foreshadowing } from "@/types"

const mockedNarrative = vi.mocked(narrativeApi)
const mockedStoryline = vi.mocked(storylineApi)
const mockedForeshadow = vi.mocked(foreshadowApi)

function makeNode(overrides: Partial<NarrativeNode>): NarrativeNode {
  return {
    id: "n",
    project_id: "p",
    world_id: "w",
    node_type: "Scene",
    title: "node",
    attributes: {},
    sort_order: 0,
    status: "Planned",
    created_at: "",
    updated_at: "",
    ...overrides,
  }
}

function makeStoryline(id: string): Storyline {
  return {
    id,
    project_id: "p",
    name: `线-${id}`,
    status: "Active",
    importance: "Normal",
    created_at: "",
    updated_at: "",
  }
}

function makeForeshadow(id: string): Foreshadowing {
  return {
    id,
    project_id: "p",
    name: `伏笔-${id}`,
    status: "Active",
    importance: "Normal",
    hint_level: "Subtle",
    related_entity_ids: [],
    created_at: "",
    updated_at: "",
  }
}

beforeEach(() => {
  setActivePinia(createPinia())
  vi.clearAllMocks()
})

describe("story store · tree 构建", () => {
  it("按 parent_id 组装层级，孤儿节点归为根", () => {
    const store = useStoryStore()
    store.nodes = [
      makeNode({ id: "vol1", node_type: "Volume", sort_order: 2 }),
      makeNode({ id: "ch1", parent_id: "vol1", sort_order: 1 }),
      makeNode({ id: "sc1", parent_id: "ch1", sort_order: 1 }),
      makeNode({ id: "orphan", parent_id: "ghost", sort_order: 0 }),
    ]

    const tree = store.tree
    expect(tree).toHaveLength(2) // vol1 + orphan
    const vol = tree.find(n => n.id === "vol1")!
    expect(vol.children.map(c => c.id)).toEqual(["ch1"])
    expect(vol.children[0].children.map(c => c.id)).toEqual(["sc1"])
  })

  it("同级节点按 sort_order 排序", () => {
    const store = useStoryStore()
    store.nodes = [
      makeNode({ id: "b", sort_order: 2 }),
      makeNode({ id: "a", sort_order: 1 }),
    ]
    expect(store.tree.map(n => n.id)).toEqual(["a", "b"])
  })
})

describe("story store · CRUD 本地同步", () => {
  it("deleteNode 调 API 后从本地移除", async () => {
    mockedNarrative.deleteNode.mockResolvedValue(undefined as any)
    const store = useStoryStore()
    store.nodes = [makeNode({ id: "keep" }), makeNode({ id: "drop" })]

    await store.deleteNode("drop")

    expect(mockedNarrative.deleteNode).toHaveBeenCalledWith("drop")
    expect(store.nodes.map(n => n.id)).toEqual(["keep"])
  })

  it("deleteStoryline / deleteForeshadow 只删目标项", async () => {
    mockedStoryline.delete.mockResolvedValue(undefined as any)
    mockedForeshadow.delete.mockResolvedValue(undefined as any)

    const store = useStoryStore()
    store.storylines = [makeStoryline("s1"), makeStoryline("s2")]
    store.foreshadows = [makeForeshadow("f1")]

    await store.deleteStoryline("s1")
    await store.deleteForeshadow("f1")

    expect(store.storylines.map(s => s.id)).toEqual(["s2"])
    expect(store.foreshadows).toHaveLength(0)
  })

  it("fetchNodes 失败时置空并记录 error", async () => {
    mockedNarrative.listNodes.mockRejectedValue(new Error("boom"))
    const store = useStoryStore()

    await store.fetchNodes("p")

    expect(store.nodes).toEqual([])
    expect(store.error).toBe("boom")
    expect(store.loading).toBe(false)
  })

  it("createNode 成功后追加到本地列表", async () => {
    const created = makeNode({ id: "new" })
    mockedNarrative.createNode.mockResolvedValue(created)
    const store = useStoryStore()

    await store.createNode("p", { title: "新节点" })

    expect(store.nodes).toContainEqual(created)
  })
})
