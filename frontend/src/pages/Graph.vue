<template>
  <div class="graph-page">
    <div class="page-header">
      <h1 class="page-title">关系图谱</h1>
      <div class="legend">
        <span v-for="l in legend" :key="l.type" class="legend-item">
          <span class="legend-dot" :style="{ background: l.color }"></span>
          {{ l.label }}
        </span>
      </div>
    </div>

    <div v-if="!loading && nodes.length === 0" class="empty-state">
      <span class="empty-icon">🕸️</span>
      <span class="empty-text">暂无实体或可关系，请先在「人物 / 地点 / 势力」中添加数据</span>
    </div>

    <div class="graph-container">
      <div
        class="graph-canvas"
        :style="{ transform: `scale(${scale})`, transformOrigin: 'center center' }"
      >
        <svg class="graph-svg" :viewBox="`0 0 ${CANVAS_SIZE} ${CANVAS_SIZE}`" :width="CANVAS_SIZE" :height="CANVAS_SIZE">
          <!-- Edges -->
          <line
            v-for="edge in visibleEdges"
            :key="edge.id"
            :x1="nodePos[edge.from]?.x ?? 0"
            :y1="nodePos[edge.from]?.y ?? 0"
            :x2="nodePos[edge.to]?.x ?? 0"
            :y2="nodePos[edge.to]?.y ?? 0"
            stroke="rgba(255,255,255,0.15)"
            stroke-width="1.5"
          />
          <!-- Edge labels -->
          <text
            v-for="edge in visibleEdges"
            :key="edge.id + '-label'"
            :x="((nodePos[edge.from]?.x ?? 0) + (nodePos[edge.to]?.x ?? 0)) / 2"
            :y="((nodePos[edge.from]?.y ?? 0) + (nodePos[edge.to]?.y ?? 0)) / 2 - 6"
            fill="rgba(255,255,255,0.4)"
            font-size="10"
            text-anchor="middle"
          >{{ edge.label }}</text>
          <!-- Nodes -->
          <g
            v-for="node in visibleNodes"
            :key="node.id"
            class="graph-node"
            @click="selectNode(node)"
          >
            <circle
              :cx="node.x"
              :cy="node.y"
              :r="node.type === 'Character' ? 28 : 24"
              :fill="nodeColors[node.type] || '#333'"
              :stroke="selectedNode?.id === node.id ? '#C84B31' : 'rgba(255,255,255,0.2)'"
              :stroke-width="selectedNode?.id === node.id ? 2 : 1"
              opacity="0.9"
            />
            <text
              :x="node.x"
              :y="node.y + 4"
              fill="white"
              font-size="12"
              text-anchor="middle"
              font-weight="500"
            >{{ node.name.slice(0, 3) }}</text>
            <text
              :x="node.x"
              :y="node.y + 40"
              fill="rgba(255,255,255,0.6)"
              font-size="11"
              text-anchor="middle"
            >{{ node.name }}</text>
          </g>
        </svg>
      </div>

      <!-- Node Inspector -->
      <div class="graph-inspector" v-if="selectedNode">
        <div class="inspector-header">
          <span class="inspector-type">{{ selectedNode.type }}</span>
          <span class="inspector-name">{{ selectedNode.name }}</span>
          <button class="close-btn" @click="selectedNode = null">×</button>
        </div>
        <div class="inspector-body">
          <div class="inspector-row" v-if="selectedNode.summary">
            <span class="row-label">简介</span>
            <span class="row-value">{{ selectedNode.summary }}</span>
          </div>
          <div class="inspector-section">
            <div class="section-title">关系</div>
            <div v-for="rel in getNodeRelations(selectedNode.id)" :key="rel.id" class="relation-item">
              <span>{{ getNodeById(rel.from)?.name || rel.from }}</span>
              <span class="rel-arrow">→</span>
              <span class="rel-label">{{ rel.label }}</span>
              <span class="rel-arrow">→</span>
              <span>{{ getNodeById(rel.to)?.name || rel.to }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <GraphControls
      :zoom="scale"
      :active-filter="activeFilter"
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

const route = useRoute()
const worldStore = useWorldStore()
const projectId = route.params.id as string
const worldId = computed(() => worldStore.currentWorld?.id ?? '')

const CANVAS_SIZE = 900
const loading = ref(false)
const activeFilter = ref('all')
const selectedNode = ref<GraphNode | null>(null)
const scale = ref(1)

const nodeColors: Record<string, string> = {
  Character: '#2D5A8E',
  Location: '#4A7C59',
  Faction: '#8B4513',
}

const legend = [
  { type: 'Character', label: '人物', color: nodeColors.Character },
  { type: 'Location', label: '地点', color: nodeColors.Location },
  { type: 'Faction', label: '势力', color: nodeColors.Faction },
]

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
  label: string
}

// Combine the three entity collections into one node array, tagging the type
// by which store slice each entity belongs to (entities expose entity_type_id,
// but the slice determines the graph type unambiguously).
const allNodes = computed<GraphNode[]>(() => {
  const circle = (entities: Entity[], type: string): GraphNode[] =>
    entities.map((e) => ({ ...toNode(e), type }))
  return [
    ...circle(worldStore.characters, 'Character'),
    ...circle(worldStore.locations, 'Location'),
    ...circle(worldStore.factions, 'Faction'),
  ]
})

const allEdges = computed<GraphEdge[]>(() =>
  worldStore.relations.map((r: Relation) => ({
    id: r.id,
    from: r.source_entity_id,
    to: r.target_entity_id,
    label: r.relation_type,
  })),
)

// Deterministic layout: arrange every node on a circle by index. The API
// provides no x/y, so this gives stable, evenly-spaced positions.
const nodePos = computed<Record<string, { x: number; y: number }>>(() => {
  const list = allNodes.value
  const cx = CANVAS_SIZE / 2
  const cy = CANVAS_SIZE / 2
  const radius = CANVAS_SIZE / 2 - 80
  const pos: Record<string, { x: number; y: number }> = {}
  list.forEach((n, i) => {
    const angle = (2 * Math.PI * i) / Math.max(list.length, 1)
    pos[n.id] = {
      x: cx + radius * Math.cos(angle),
      y: cy + radius * Math.sin(angle),
    }
  })
  return pos
})

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

function toNode(e: Entity): GraphNode {
  return {
    id: e.id,
    name: e.name,
    type: e.entity_type_id,
    summary: e.summary,
    x: 0,
    y: 0,
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

const MIN_SCALE = 0.3
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
      await worldStore.fetchCharacters(worldId.value)
      await worldStore.fetchLocations(worldId.value)
      await worldStore.fetchFactions(worldId.value)
      await worldStore.fetchRelations(worldId.value)
    }
  } finally {
    loading.value = false
  }
})
</script>

<style scoped>
.graph-page { height: 100%; display: flex; flex-direction: column; overflow: hidden; }
.page-header { display: flex; justify-content: space-between; align-items: center; padding: var(--space-4) var(--space-6); border-bottom: 1px solid var(--border-default); flex-shrink: 0; }
.page-title { font-size: var(--text-xl); font-weight: 700; font-family: var(--font-serif); }

.legend { display: flex; gap: var(--space-4); }
.legend-item { display: flex; align-items: center; gap: var(--space-1); font-size: var(--text-xs); color: var(--text-secondary); }
.legend-dot { width: 10px; height: 10px; border-radius: 50%; display: inline-block; }

.graph-container { flex: 1; position: relative; overflow: hidden; }
.graph-canvas { width: 100%; height: 100%; display: flex; align-items: center; justify-content: center; transition: transform var(--transition-fast); }
.graph-svg { max-width: 100%; max-height: 100%; }
.graph-node { cursor: pointer; }

.empty-state { flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; color: var(--text-tertiary); }
.empty-icon { font-size: 48px; margin-bottom: var(--space-4); }
.empty-text { font-size: var(--text-sm); }

.graph-inspector {
  position: absolute;
  top: var(--space-4);
  right: var(--space-4);
  width: 280px;
  background: var(--bg-panel);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-lg);
}

.inspector-header { display: flex; align-items: center; gap: var(--space-2); padding: var(--space-3) var(--space-4); border-bottom: 1px solid var(--border-muted); }
.inspector-type { font-size: 10px; padding: 2px 6px; background: var(--bg-panel-secondary); border-radius: 3px; color: var(--text-tertiary); }
.inspector-name { font-weight: 600; }
.close-btn { margin-left: auto; border: none; background: transparent; color: var(--text-tertiary); cursor: pointer; font-size: var(--text-lg); }

.inspector-body { padding: var(--space-3) var(--space-4); }
.inspector-row { margin-bottom: var(--space-2); }
.row-label { font-size: var(--text-xs); color: var(--text-tertiary); display: block; margin-bottom: var(--space-1); }
.row-value { font-size: var(--text-sm); color: var(--text-secondary); }

.inspector-section { margin-top: var(--space-3); }
.section-title { font-size: var(--text-xs); font-weight: 600; text-transform: uppercase; color: var(--text-tertiary); margin-bottom: var(--space-2); }
.relation-item { display: flex; align-items: center; gap: var(--space-2); font-size: var(--text-xs); color: var(--text-secondary); padding: var(--space-1) 0; }
.rel-arrow { color: var(--text-tertiary); }
.rel-label { color: var(--color-primary-text); }
</style>
