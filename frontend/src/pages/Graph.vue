<template>
  <div class="graph-page">
    <div class="page-header">
      <h1 class="page-title">关系图谱</h1>
      <div class="graph-controls">
        <button v-for="filter in filters" :key="filter.id" class="filter-btn" :class="{ active: activeFilter === filter.id }" @click="activeFilter = filter.id">
          {{ filter.label }}
        </button>
      </div>
    </div>

    <div class="graph-container">
      <svg class="graph-svg" viewBox="0 0 800 600">
        <!-- Edges -->
        <line v-for="edge in edges" :key="edge.id"
          :x1="getNodePos(edge.from).x" :y1="getNodePos(edge.from).y"
          :x2="getNodePos(edge.to).x" :y2="getNodePos(edge.to).y"
          stroke="rgba(255,255,255,0.15)" stroke-width="1.5"
        />
        <!-- Edge labels -->
        <text v-for="edge in edges" :key="edge.id + '-label'"
          :x="(getNodePos(edge.from).x + getNodePos(edge.to).x) / 2"
          :y="(getNodePos(edge.from).y + getNodePos(edge.to).y) / 2 - 6"
          fill="rgba(255,255,255,0.4)" font-size="10" text-anchor="middle"
        >{{ edge.label }}</text>
        <!-- Nodes -->
        <g v-for="node in nodes" :key="node.id" class="graph-node" @click="selectNode(node)">
          <circle
            :cx="node.x" :cy="node.y" :r="node.type === 'Character' ? 28 : 24"
            :fill="nodeColors[node.type] || '#333'"
            :stroke="selectedNode?.id === node.id ? '#C84B31' : 'rgba(255,255,255,0.2)'"
            :stroke-width="selectedNode?.id === node.id ? 2 : 1"
            opacity="0.9"
          />
          <text :x="node.x" :y="node.y + 4" fill="white" font-size="12" text-anchor="middle" font-weight="500">
            {{ node.name.slice(0, 3) }}
          </text>
          <text :x="node.x" :y="node.y + 40" fill="rgba(255,255,255,0.6)" font-size="11" text-anchor="middle">
            {{ node.name }}
          </text>
        </g>
      </svg>

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
    <GraphControls :zoom="1" active-filter="all" @zoom-in="zoomIn" @zoom-out="zoomOut" @zoom-reset="zoomReset" @fit="fitGraph" @center="centerGraph" />
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useWorldStore } from '@/stores/world'

const worldStore = useWorldStore()

import GraphControls from "@/components/graph/GraphControls.vue"

const activeFilter = ref('all')
const selectedNode = ref<any>(null)

const filters = [
  { id: 'all', label: '全部' },
  { id: 'Character', label: '人物' },
  { id: 'Location', label: '地点' },
  { id: 'Faction', label: '势力' },
]

const nodeColors: Record<string, string> = {
  Character: '#2D5A8E',
  Location: '#4A7C59',
  Faction: '#8B4513',
}

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

const nodes = computed<GraphNode[]>(() => {
  const allNodes: GraphNode[] = [
    // Characters
    { id: 'char-1', name: '林凡', type: 'Character', summary: '主角，边境散修', x: 300, y: 200 },
    { id: 'char-2', name: '苏晚晴', type: 'Character', summary: '女主，神秘女子', x: 500, y: 200 },
    { id: 'char-3', name: '王天德', type: 'Character', summary: '王家家主', x: 200, y: 350 },
    // Locations
    { id: 'loc-1', name: '黑石城', type: 'Location', summary: '北境重镇', x: 400, y: 400 },
    { id: 'loc-2', name: '地下遗迹', type: 'Location', summary: '远古遗迹', x: 600, y: 350 },
    { id: 'loc-3', name: '古井', type: 'Location', summary: '神秘古井', x: 550, y: 450 },
    // Factions
    { id: 'fac-1', name: '王家', type: 'Faction', summary: '四大家族之首', x: 150, y: 250 },
    { id: 'fac-2', name: '黑市', type: 'Faction', summary: '地下势力', x: 650, y: 250 },
  ]

  if (activeFilter.value !== 'all') {
    return allNodes.filter(n => n.type === activeFilter.value)
  }
  return allNodes
})

const edges = computed<GraphEdge[]>(() => {
  const allEdges: GraphEdge[] = [
    { id: 'e1', from: 'char-1', to: 'char-2', label: '同伴' },
    { id: 'e2', from: 'char-3', to: 'char-1', label: '追杀' },
    { id: 'e3', from: 'char-1', to: 'loc-1', label: '位于' },
    { id: 'e4', from: 'char-1', to: 'loc-2', label: '探索' },
    { id: 'e5', from: 'char-2', to: 'loc-2', label: '同行' },
    { id: 'e6', from: 'char-3', to: 'fac-1', label: '领导' },
    { id: 'e7', from: 'fac-1', to: 'loc-1', label: '控制' },
    { id: 'e8', from: 'char-1', to: 'loc-3', label: '发现' },
    { id: 'e9', from: 'fac-2', to: 'loc-1', label: '隐藏于' },
  ]

  const nodeIds = new Set(nodes.value.map(n => n.id))
  return allEdges.filter(e => nodeIds.has(e.from) && nodeIds.has(e.to))
})

function getNodePos(id: string) {
  const node = nodes.value.find(n => n.id === id)
  return node ? { x: node.x, y: node.y } : { x: 0, y: 0 }
}

function selectNode(node: any) {
  selectedNode.value = selectedNode.value?.id === node.id ? null : node
}

function getNodeRelations(id: string) {
  return edges.value.filter(e => e.from === id || e.to === id)
}

function zoomIn() { console.log('zoom in') }
function zoomOut() { console.log('zoom out') }
function zoomReset() { console.log('zoom reset') }
function fitGraph() { console.log('fit') }
function centerGraph() { console.log('center') }

function getNodeById(id: string) {
  return nodes.value.find(n => n.id === id)
}
</script>

<style scoped>
.graph-page { height: 100%; display: flex; flex-direction: column; overflow: hidden; }
.page-header { display: flex; justify-content: space-between; align-items: center; padding: var(--space-4) var(--space-6); border-bottom: 1px solid var(--border-default); flex-shrink: 0; }
.page-title { font-size: var(--text-xl); font-weight: 700; font-family: var(--font-serif); }
.graph-controls { display: flex; gap: var(--space-2); }
.filter-btn { padding: var(--space-1) var(--space-3); border: 1px solid var(--border-default); background: transparent; color: var(--text-secondary); border-radius: var(--radius-sm); font-size: var(--text-xs); cursor: pointer; transition: all var(--transition-fast); }
.filter-btn.active { background: var(--color-primary-subtle); border-color: var(--color-primary); color: var(--color-primary-text); }

.graph-container { flex: 1; position: relative; overflow: hidden; }
.graph-svg { width: 100%; height: 100%; }
.graph-node { cursor: pointer; }

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