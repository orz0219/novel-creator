<template>
  <div class="graph-page">
    <div class="page-header">
      <div class="header-title">
        <h1 class="page-title">关系图谱</h1>
        <p class="page-subtitle">点击节点查看它以哪些关系连接到世界上的其他实体</p>
      </div>
      <div class="legend">
        <span v-for="l in legendTypes" :key="l.type" class="legend-item">
          <span class="legend-dot" :style="{ background: l.color }"></span>
          {{ l.label }}
          <span class="legend-count">{{ l.count }}</span>
        </span>
      </div>
    </div>

    <div v-if="!loading && nodes.length === 0" class="empty-state">
      <Network class="empty-icon" :size="40" />
      <span class="empty-text">暂无实体或关系，请先在人物 / 地点 / 势力 / 物品中添加数据</span>
    </div>

    <div v-else class="graph-container">
      <div
        class="graph-canvas"
        :style="{ transform: `scale(${scale})`, transformOrigin: 'center center' }"
      >
        <svg
          class="graph-svg"
          :viewBox="`0 0 ${CANVAS_SIZE} ${CANVAS_SIZE}`"
          :width="CANVAS_SIZE"
          :height="CANVAS_SIZE"
        >
          <!-- 边 -->
          <g class="edge-layer">
            <line
              v-for="edge in visibleEdges"
              :key="edge.id"
              :x1="nodePos[edge.from]?.x ?? 0"
              :y1="nodePos[edge.from]?.y ?? 0"
              :x2="nodePos[edge.to]?.x ?? 0"
              :y2="nodePos[edge.to]?.y ?? 0"
              class="graph-edge"
              :class="{
                'is-active': isEdgeActive(edge),
                'is-dimmed': !isEdgeActive(edge),
              }"
            />
          </g>

          <!-- 关系标签 -->
          <g class="edge-label-layer">
            <text
              v-for="edge in visibleEdges"
              :key="edge.id + '-label'"
              :x="edgeMidpoint(edge).x"
              :y="edgeMidpoint(edge).y - 6"
              class="edge-label"
              :class="{ 'is-dimmed': !isEdgeActive(edge) }"
              text-anchor="middle"
            >{{ edge.label }}</text>
          </g>

          <!-- 节点 -->
          <g class="node-layer">
            <g
              v-for="node in visibleNodes"
              :key="node.id"
              class="graph-node"
              :class="{ 'is-selected': selectedNode?.id === node.id, 'is-dimmed': !isNodeActive(node) }"
              @click="selectNode(node)"
            >
              <circle
                :cx="node.x"
                :cy="node.y"
                :r="nodeRadius(node)"
                :fill="entityTypeColor(node.type)"
                :stroke="selectedNode?.id === node.id ? '#FFFFFF' : 'rgba(255,255,255,0.18)'"
                :stroke-width="selectedNode?.id === node.id ? 2.5 : 1"
              />
              <text
                :x="node.x"
                :y="node.y + 4"
                class="node-initial"
                text-anchor="middle"
              >{{ nodeInitial(node.name) }}</text>
              <text
                :x="node.x"
                :y="node.y + nodeRadius(node) + 15"
                class="node-name"
                text-anchor="middle"
              >{{ truncate(node.name, 12) }}</text>
            </g>
          </g>
        </svg>
      </div>

      <!-- 节点详情 -->
      <div class="graph-inspector" v-if="selectedNode">
        <div class="inspector-header" :style="{ borderLeftColor: entityTypeColor(selectedNode.type) }">
          <div>
            <span class="inspector-type">{{ entityTypeLabel(selectedNode.type) }}</span>
            <span class="inspector-name">{{ selectedNode.name }}</span>
          </div>
          <button class="close-btn" @click="selectedNode = null">×</button>
        </div>
        <div class="inspector-body">
          <div class="inspector-row" v-if="selectedNode.summary">
            <span class="row-label">简介</span>
            <span class="row-value">{{ selectedNode.summary }}</span>
          </div>
          <div class="inspector-section">
            <div class="section-title">关系（{{ getNodeRelations(selectedNode.id).length }}）</div>
            <div v-if="getNodeRelations(selectedNode.id).length === 0" class="relation-empty">
              这个节点目前没有连接关系
            </div>
            <div v-for="rel in getNodeRelations(selectedNode.id)" :key="rel.id" class="relation-item">
              <span class="rel-node">{{ getNodeById(rel.from)?.name || '未知' }}</span>
              <span class="rel-arrow">→</span>
              <span class="rel-label" :title="rel.description || rel.rawLabel">{{ rel.label }}</span>
              <span class="rel-arrow">→</span>
              <span class="rel-node">{{ getNodeById(rel.to)?.name || '未知' }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <GraphControls
      :zoom="scale"
      :active-filter="activeFilter"
      :types="filterTypes"
      @zoom-in="zoomIn"
      @zoom-out="zoomOut"
      @zoom-reset="zoomReset"
      @fit="fitGraph"
      @center="centerGraph"
      @filter="activeFilter = $event"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { useWorldStore } from '@/stores/world'
import GraphControls from '@/components/graph/GraphControls.vue'
import type { Entity, Relation } from '@/types/world'
import { Network } from 'lucide-vue-next'
import { computeForceLayout } from '@/utils/graphLayout'
import {
  relationLabel,
  entityTypeLabel,
  entityTypeColor,
} from '@/utils/relationDisplay'

const route = useRoute()
const worldStore = useWorldStore()
const projectId = route.params.id as string
const worldId = computed(() => worldStore.currentWorld?.id ?? '')

const CANVAS_SIZE = 1000
const loading = ref(false)
const activeFilter = ref('all')
const selectedNode = ref<GraphNode | null>(null)
const scale = ref(1)

interface GraphNode {
  id: string
  name: string
  type: string
  summary?: string
  x: number
  y: number
}

interface GraphEdge {
  id: string
  from: string
  to: string
  rawLabel: string
  label: string
  description?: string
}

const graphEntities = ref<Entity[]>([])

/** 所有节点：从当前世界的所有实体中取，因此人物 / 地点 / 势力 / 物品等都会进图。 */
const allNodes = computed<GraphNode[]>(() =>
  graphEntities.value.map((e) => ({
    id: e.id,
    name: e.name,
    type: e.entity_type_id || 'Unknown',
    summary: e.summary,
    x: 0,
    y: 0,
  })),
)

const nodeTypeById = computed(() => {
  const map = new Map<string, string>()
  for (const node of allNodes.value) map.set(node.id, node.type)
  return map
})

/** 关系边：关系类型统一走展示翻译，不再直接把 enemy / took_life 丢到界面上。 */
const allEdges = computed<GraphEdge[]>(() =>
  worldStore.relations
    .filter((r: Relation) => nodeTypeById.value.has(r.source_entity_id) && nodeTypeById.value.has(r.target_entity_id))
    .map((r: Relation) => ({
      id: r.id,
      from: r.source_entity_id,
      to: r.target_entity_id,
      rawLabel: r.relation_type,
      label: relationLabel(r.relation_type),
      description: r.description,
    })),
)

// 布局基于全部节点/边，筛选时只过滤显示，避免切换筛选位置跳变。
const nodePos = computed(() =>
  computeForceLayout(allNodes.value, allEdges.value, {
    width: CANVAS_SIZE,
    height: CANVAS_SIZE,
  }),
)

const nodes = computed<GraphNode[]>(() =>
  allNodes.value.map((n) => ({ ...n, ...nodePos.value[n.id] })),
)

const visibleNodes = computed<GraphNode[]>(() =>
  activeFilter.value === 'all'
    ? nodes.value
    : nodes.value.filter((n) => n.type === activeFilter.value),
)

const visibleEdges = computed<GraphEdge[]>(() => {
  const ids = new Set(visibleNodes.value.map((n) => n.id))
  return allEdges.value.filter((e) => ids.has(e.from) && ids.has(e.to))
})

/** 右侧筛选按钮只展示当前图里真实存在的类型。 */
const filterTypes = computed(() => {
  const seen = new Map<string, { type: string; label: string; count: number }>()
  for (const node of allNodes.value) {
    const current = seen.get(node.type)
    if (current) current.count += 1
    else seen.set(node.type, { type: node.type, label: entityTypeLabel(node.type), count: 1 })
  }
  return [...seen.values()].sort((a, b) => b.count - a.count)
})

const legendTypes = computed(() =>
  filterTypes.value.map((t) => ({
    ...t,
    color: entityTypeColor(t.type),
  })),
)

const selectedNeighborIds = computed(() => {
  const set = new Set<string>()
  const id = selectedNode.value?.id
  if (!id) return set
  for (const edge of allEdges.value) {
    if (edge.from === id) set.add(edge.to)
    if (edge.to === id) set.add(edge.from)
  }
  return set
})

const degreeById = computed(() => {
  const map = new Map<string, number>()
  for (const edge of allEdges.value) {
    map.set(edge.from, (map.get(edge.from) ?? 0) + 1)
    map.set(edge.to, (map.get(edge.to) ?? 0) + 1)
  }
  return map
})

function nodeRadius(node: GraphNode): number {
  return Math.min(34, 18 + (degreeById.value.get(node.id) ?? 0) * 1.8)
}

function nodeInitial(name: string): string {
  return name.trim().slice(0, 2) || '?'
}

function truncate(text: string, max: number): string {
  return text.length > max ? `${text.slice(0, max)}…` : text
}

function isNodeActive(node: GraphNode): boolean {
  if (!selectedNode.value) return true
  return node.id === selectedNode.value.id || selectedNeighborIds.value.has(node.id)
}

function isEdgeActive(edge: GraphEdge): boolean {
  if (!selectedNode.value) return true
  return edge.from === selectedNode.value.id || edge.to === selectedNode.value.id
}

function edgeMidpoint(edge: GraphEdge) {
  const from = nodePos.value[edge.from]
  const to = nodePos.value[edge.to]
  return {
    x: ((from?.x ?? 0) + (to?.x ?? 0)) / 2,
    y: ((from?.y ?? 0) + (to?.y ?? 0)) / 2,
  }
}

function getNodeById(id: string) {
  return nodes.value.find((n) => n.id === id)
}

function getNodeRelations(id: string) {
  return visibleEdges.value.filter((e) => e.from === id || e.to === id)
}

function selectNode(node: GraphNode) {
  selectedNode.value = selectedNode.value?.id === node.id ? null : node
}

const MIN_SCALE = 0.35
const MAX_SCALE = 3
function zoomIn() {
  scale.value = Math.min(MAX_SCALE, scale.value + 0.1)
}
function zoomOut() {
  scale.value = Math.max(MIN_SCALE, scale.value - 0.1)
}
function zoomReset() {
  scale.value = 1
}
function fitGraph() {
  scale.value = 1
}
function centerGraph() {
  scale.value = 1
}

onMounted(async () => {
  loading.value = true
  try {
    if (!worldStore.currentWorld) await worldStore.fetchWorld(projectId)
    if (worldId.value) {
      const [entities] = await Promise.all([
        worldStore.fetchEntities(worldId.value),
        worldStore.fetchRelations(worldId.value),
      ])
      graphEntities.value = entities
    }
  } finally {
    loading.value = false
  }
})
</script>

<style scoped>
.graph-page { height: 100%; display: flex; flex-direction: column; overflow: hidden; background: var(--bg-base); }
.page-header { display: flex; justify-content: space-between; align-items: center; gap: var(--space-4); padding: var(--space-4) var(--space-6); border-bottom: 1px solid var(--border-default); flex-shrink: 0; }
.header-title { min-width: 0; }
.page-title { font-size: var(--text-xl); font-weight: 700; font-family: var(--font-serif); }
.page-subtitle { margin-top: 2px; font-size: var(--text-xs); color: var(--text-tertiary); }

.legend { display: flex; flex-wrap: wrap; justify-content: flex-end; gap: var(--space-2) var(--space-4); }
.legend-item { display: flex; align-items: center; gap: var(--space-1); font-size: var(--text-xs); color: var(--text-secondary); }
.legend-dot { width: 9px; height: 9px; border-radius: 50%; display: inline-block; }
.legend-count { color: var(--text-tertiary); font-size: 10px; }

.graph-container { position: relative; flex: 1; overflow: hidden; }
.graph-canvas { width: 100%; height: 100%; display: flex; align-items: center; justify-content: center; transition: transform var(--transition-fast); }
.graph-svg { max-width: 100%; max-height: 100%; }

.graph-edge { stroke: rgba(255, 255, 255, 0.12); stroke-width: 1.4; transition: all var(--transition-fast); }
.graph-edge.is-active { stroke: rgba(255, 255, 255, 0.42); stroke-width: 1.7; }
.graph-edge.is-dimmed { stroke: rgba(255, 255, 255, 0.06); }

.edge-label { fill: rgba(255, 255, 255, 0.55); font-size: 11px; paint-order: stroke; stroke: var(--bg-base); stroke-width: 3px; stroke-linejoin: round; transition: opacity var(--transition-fast); }
.edge-label.is-dimmed { opacity: 0.12; }

.graph-node { cursor: pointer; transition: opacity var(--transition-fast); }
.graph-node.is-dimmed { opacity: 0.22; }
.graph-node circle { transition: stroke var(--transition-fast), stroke-width var(--transition-fast); }
.graph-node:hover circle { stroke: rgba(255, 255, 255, 0.65); }
.node-initial { fill: #fff; font-size: 12px; font-weight: 700; pointer-events: none; }
.node-name { fill: var(--text-secondary); font-size: 11px; pointer-events: none; paint-order: stroke; stroke: var(--bg-base); stroke-width: 3px; stroke-linejoin: round; }

.graph-inspector {
  position: absolute; top: var(--space-4); right: var(--space-4); width: 300px; max-height: calc(100% - var(--space-8));
  display: flex; flex-direction: column;
  background: var(--bg-panel); border: 1px solid var(--border-default); border-radius: var(--radius-md);
  box-shadow: var(--shadow-lg); overflow: hidden;
}
.inspector-header { display: flex; align-items: center; justify-content: space-between; gap: var(--space-2); padding: var(--space-3) var(--space-4); border-bottom: 1px solid var(--border-muted); border-left: 3px solid var(--color-primary); }
.inspector-type { display: block; font-size: 10px; color: var(--text-tertiary); margin-bottom: 2px; }
.inspector-name { display: block; font-weight: 600; font-family: var(--font-serif); overflow-wrap: anywhere; }
.close-btn { border: none; background: transparent; color: var(--text-tertiary); cursor: pointer; font-size: var(--text-lg); }
.close-btn:hover { color: var(--text-primary); }

.inspector-body { padding: var(--space-3) var(--space-4); overflow-y: auto; }
.inspector-row { margin-bottom: var(--space-2); }
.row-label { display: block; margin-bottom: var(--space-1); font-size: var(--text-xs); color: var(--text-tertiary); }
.row-value { font-size: var(--text-sm); color: var(--text-secondary); line-height: 1.6; }
.inspector-section { margin-top: var(--space-3); }
.section-title { margin-bottom: var(--space-2); font-size: var(--text-xs); font-weight: 600; color: var(--text-tertiary); }
.relation-empty { font-size: var(--text-xs); color: var(--text-disabled); padding: var(--space-2) 0; }
.relation-item { display: flex; align-items: center; flex-wrap: wrap; gap: var(--space-1); padding: 5px 0; border-bottom: 1px dashed var(--border-muted); font-size: var(--text-xs); color: var(--text-secondary); }
.relation-item:last-child { border-bottom: none; }
.rel-node { color: var(--text-primary); max-width: 88px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.rel-arrow { color: var(--text-tertiary); }
.rel-label { color: var(--color-primary-text); background: var(--color-primary-subtle); border-radius: 4px; padding: 1px 5px; }

.empty-state { flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; color: var(--text-tertiary); }
.empty-icon { margin-bottom: var(--space-4); }
.empty-text { font-size: var(--text-sm); }
</style>
