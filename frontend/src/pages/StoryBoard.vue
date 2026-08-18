<template>
  <div class="storyboard-page">
    <div class="page-header">
      <h1 class="page-title">故事看板</h1>
    </div>
    <div class="board-columns">
      <div v-for="col in columns" :key="col.id" class="board-column">
        <div class="column-header">
          <span class="column-title">{{ col.title }}</span>
          <span class="column-count">{{ col.items.length }}</span>
        </div>
        <div class="column-body">
          <div v-for="item in col.items" :key="item.id" class="board-card">
            <div class="card-title">{{ item.title }}</div>
            <div class="card-desc" v-if="item.description">{{ item.description }}</div>
            <div class="card-meta">
              <span class="card-status" :class="item.status">{{ item.status }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
const columns = [
  { id: 'planned', title: '待规划', items: [
    { id: 'ch-5', title: '第五章：逃离黑石城', description: '林凡被迫离开黑石城', status: 'Planned' },
  ]},
  { id: 'writing', title: '创作中', items: [
    { id: 'scene-1', title: '场景1：遗迹入口', description: '林凡找到地下遗迹的入口', status: 'InProgress' },
    { id: 'scene-2', title: '场景2：机关重重', description: '林凡和苏晚晴遭遇机关', status: 'Draft' },
  ]},
  { id: 'review', title: '待审核', items: [
    { id: 'ch-3', title: '第三章：暗流涌动', description: '王家开始注意到林凡', status: 'Completed' },
  ]},
  { id: 'done', title: '已完成', items: [
    { id: 'ch-1', title: '第一章：边境来客', description: '林凡抵达黑石城', status: 'Completed' },
    { id: 'ch-2', title: '第二章：黑市风云', description: '林凡进入黑市', status: 'Completed' },
  ]},
]
</script>

<style scoped>
.storyboard-page { height: 100%; display: flex; flex-direction: column; overflow: hidden; }
.page-header { padding: var(--space-4) var(--space-6); border-bottom: 1px solid var(--border-default); flex-shrink: 0; }
.page-title { font-size: var(--text-xl); font-weight: 700; font-family: var(--font-serif); }
.board-columns { flex: 1; display: flex; gap: var(--space-4); padding: var(--space-4) var(--space-6); overflow-x: auto; }
.board-column { min-width: 260px; flex: 1; display: flex; flex-direction: column; background: var(--bg-panel); border: 1px solid var(--border-default); border-radius: var(--radius-md); overflow: hidden; }
.column-header { display: flex; justify-content: space-between; align-items: center; padding: var(--space-3) var(--space-4); border-bottom: 1px solid var(--border-muted); }
.column-title { font-size: var(--text-sm); font-weight: 600; }
.column-count { font-size: var(--text-xs); color: var(--text-tertiary); background: var(--bg-panel-secondary); padding: 2px 8px; border-radius: 10px; }
.column-body { flex: 1; padding: var(--space-2); overflow-y: auto; }
.board-card { padding: var(--space-3); border: 1px solid var(--border-muted); border-radius: var(--radius-sm); margin-bottom: var(--space-2); cursor: pointer; transition: all var(--transition-fast); }
.board-card:hover { border-color: var(--border-emphasis); background: var(--bg-hover); }
.card-title { font-size: var(--text-sm); font-weight: 500; margin-bottom: var(--space-1); }
.card-desc { font-size: var(--text-xs); color: var(--text-secondary); margin-bottom: var(--space-2); }
.card-status { font-size: 10px; padding: 2px 6px; border-radius: 3px; }
.card-status.Completed { background: var(--color-success-subtle); color: var(--color-success); }
.card-status.InProgress { background: var(--color-accent-subtle); color: var(--color-accent); }
.card-status.Planned { background: var(--bg-panel-secondary); color: var(--text-tertiary); }
.card-status.Draft { background: var(--color-warning-subtle); color: var(--color-warning); }
</style>
