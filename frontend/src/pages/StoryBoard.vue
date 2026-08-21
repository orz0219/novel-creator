<template>
  <div class="storyboard-page">
    <div class="page-header">
      <h1 class="page-title">故事看板</h1>
    </div>

    <div v-if="storyStore.loading" class="state-block">
      <span class="state-text">加载中…</span>
    </div>
    <div v-else-if="!storyStore.nodes.length" class="state-block">
      <span class="state-icon">📋</span>
      <span class="state-text">暂无节点</span>
    </div>

    <div v-else class="board-columns">
      <div v-for="col in columns" :key="col.status" class="board-column">
        <div class="column-header">
          <span class="column-title">{{ col.label }}</span>
          <span class="column-count">{{ col.nodes.length }}</span>
        </div>
        <div class="column-body">
          <component
            :is="isWritable(col.nodes[idx]) ? 'router-link' : 'div'"
            v-for="(node, idx) in col.nodes"
            :key="node.id"
            :to="isWritable(node) ? `/project/${projectId}/write/${node.id}` : undefined"
            class="board-card"
          >
            <div class="card-title">{{ node.title }}</div>
            <div class="card-desc" v-if="node.description">{{ node.description }}</div>
            <div class="card-meta">
              <span class="status-dot" :class="node.status"></span>
              <span class="card-status" :class="node.status">{{ col.label }}</span>
            </div>
          </component>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useRoute } from 'vue-router'
import { useStoryStore } from '@/stores/story'
import { computed, onMounted } from 'vue'
import type { NarrativeNode, NarrativeNodeStatus } from '@/types'

const route = useRoute()
const storyStore = useStoryStore()
const projectId = route.params.id as string

const STATUS_ORDER: NarrativeNodeStatus[] = ['Draft', 'Planned', 'InProgress', 'Completed', 'Archived']
const STATUS_LABELS: Record<NarrativeNodeStatus, string> = {
  Draft: '草稿',
  Planned: '已规划',
  InProgress: '进行中',
  Completed: '已完成',
  Archived: '已归档',
}

const columns = computed(() =>
  STATUS_ORDER.map((status) => ({
    status,
    label: STATUS_LABELS[status],
    nodes: storyStore.nodes.filter((n: NarrativeNode) => n.status === status),
  })),
)

function isWritable(node: NarrativeNode): boolean {
  return node.node_type === 'Scene' || node.node_type === 'Chapter'
}

onMounted(async () => {
  await storyStore.fetchNodes(projectId)
})
</script>

<style scoped>
.storyboard-page { height: 100%; display: flex; flex-direction: column; overflow: hidden; }
.page-header { padding: var(--space-4) var(--space-6); border-bottom: 1px solid var(--border-default); flex-shrink: 0; }
.page-title { font-size: var(--text-2xl); font-weight: 700; font-family: var(--font-serif); }
.state-block { flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; color: var(--text-tertiary); gap: var(--space-2); }
.state-icon { font-size: 48px; }
.state-text { font-size: var(--text-sm); }
.board-columns { flex: 1; display: flex; gap: var(--space-4); padding: var(--space-4) var(--space-6); overflow-x: auto; }
.board-column { min-width: 260px; flex: 1; display: flex; flex-direction: column; background: var(--bg-panel); border: 1px solid var(--border-default); border-radius: var(--radius-md); overflow: hidden; }
.column-header { display: flex; justify-content: space-between; align-items: center; padding: var(--space-3) var(--space-4); border-bottom: 1px solid var(--border-muted); }
.column-title { font-size: var(--text-sm); font-weight: 600; }
.column-count { font-size: var(--text-xs); color: var(--text-tertiary); background: var(--bg-panel-secondary); padding: 2px 8px; border-radius: 10px; }
.column-body { flex: 1; padding: var(--space-2); overflow-y: auto; }
.board-card { display: block; padding: var(--space-3); border: 1px solid var(--border-muted); border-radius: var(--radius-sm); margin-bottom: var(--space-2); cursor: pointer; transition: all var(--transition-fast); text-decoration: none; color: inherit; }
.board-card:hover { border-color: var(--border-emphasis); background: var(--bg-hover); }
.card-title { font-size: var(--text-sm); font-weight: 500; margin-bottom: var(--space-1); }
.card-desc { font-size: var(--text-xs); color: var(--text-secondary); margin-bottom: var(--space-2); }
.card-meta { display: flex; align-items: center; gap: var(--space-2); }
.status-dot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; }
.status-dot.Completed { background: var(--color-success); }
.status-dot.InProgress { background: var(--color-accent); }
.status-dot.Planned { background: var(--text-tertiary); }
.status-dot.Draft { background: var(--color-warning); }
.status-dot.Archived { background: var(--border-emphasis); }
.card-status { font-size: 10px; padding: 2px 6px; border-radius: 3px; }
.card-status.Completed { background: var(--color-success-subtle); color: var(--color-success); }
.card-status.InProgress { background: var(--color-accent-subtle); color: var(--color-accent); }
.card-status.Planned { background: var(--bg-panel-secondary); color: var(--text-tertiary); }
.card-status.Draft { background: var(--color-warning-subtle); color: var(--color-warning); }
.card-status.Archived { background: var(--border-muted); color: var(--text-secondary); }
</style>
