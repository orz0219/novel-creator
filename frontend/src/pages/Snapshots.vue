<template>
  <div class="snapshots-page">
    <div class="page-header">
      <h1 class="page-title">世界快照</h1>
      <button class="btn-primary" @click="createSnapshot">+ 创建快照</button>
    </div>
    <div class="snapshot-list">
      <div v-for="snap in snapshots" :key="snap.id" class="snapshot-card" :class="{ current: snap.isCurrent }">
        <div class="snap-header">
          <span class="snap-id">#{{ snap.id }}</span>
          <span class="snap-name">{{ snap.name }}</span>
          <span class="snap-badge" v-if="snap.isCurrent">当前</span>
          <span class="snap-time">{{ snap.time }}</span>
        </div>
        <div class="snap-stats">
          <span class="stat">👤 {{ snap.characters }} 人物</span>
          <span class="stat">📍 {{ snap.locations }} 地点</span>
          <span class="stat">⚔️ {{ snap.factions }} 势力</span>
          <span class="stat">📖 {{ snap.chapters }} 章</span>
        </div>
        <div class="snap-meta">
          <span>时间线: {{ snap.timeline }}</span>
          <span>故事进度: {{ snap.progress }}</span>
        </div>
        <div class="snap-actions">
          <button class="action-btn">恢复</button>
          <button class="action-btn">对比</button>
          <button class="action-btn">分支</button>
          <button class="action-btn danger">删除</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
const snapshots = ref([
  {
    id: 42, name: '第三卷完成时', isCurrent: false, time: '3月10日 18:00',
    characters: 3, locations: 3, factions: 2, chapters: 3,
    timeline: '天玄历381年3月10日', progress: '第一卷·第二弧线·第三章'
  },
  {
    id: 43, name: '地下遗迹发现后', isCurrent: false, time: '3月12日 15:00',
    characters: 3, locations: 3, factions: 2, chapters: 4,
    timeline: '天玄历381年3月12日', progress: '第一卷·第二弧线·第四章'
  },
  {
    id: 44, name: '当前状态', isCurrent: true, time: '3月12日 18:30',
    characters: 3, locations: 3, factions: 2, chapters: 4,
    timeline: '天玄历381年3月12日', progress: '第一卷·第二弧线·第四章·场景2'
  },
])

function createSnapshot() {
  const nextId = Math.max(...snapshots.value.map(s => s.id)) + 1
  snapshots.value.unshift({
    id: nextId, name: '手动快照', isCurrent: true, time: '刚刚',
    characters: 3, locations: 3, factions: 2, chapters: 4,
    timeline: '天玄历381年3月12日', progress: '第一卷·第二弧线·第四章·场景2'
  })
  snapshots.value.forEach(s => { if (s.id !== nextId) s.isCurrent = false })
}
</script>

<style scoped>
.snapshots-page { height: 100%; overflow-y: auto; padding: var(--space-6) var(--space-8); }
.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--space-6); }
.page-title { font-size: var(--text-2xl); font-weight: 700; font-family: var(--font-serif); }
.btn-primary { padding: var(--space-2) var(--space-4); background: var(--color-primary); border: none; color: white; border-radius: var(--radius-sm); font-size: var(--text-sm); cursor: pointer; }
.btn-primary:hover { background: var(--color-primary-hover); }
.snapshot-list { display: flex; flex-direction: column; gap: var(--space-4); }
.snapshot-card { padding: var(--space-5); border: 1px solid var(--border-default); border-radius: var(--radius-md); background: var(--bg-panel); transition: all var(--transition-fast); }
.snapshot-card.current { border-color: var(--color-primary); background: var(--color-primary-subtle); }
.snap-header { display: flex; align-items: center; gap: var(--space-3); margin-bottom: var(--space-3); }
.snap-id { font-family: var(--font-mono); font-size: var(--text-sm); color: var(--text-tertiary); }
.snap-name { font-size: var(--text-md); font-weight: 600; }
.snap-badge { font-size: 10px; padding: 2px 8px; border-radius: 10px; background: var(--color-primary); color: white; }
.snap-time { margin-left: auto; font-size: var(--text-xs); color: var(--text-tertiary); }
.snap-stats { display: flex; gap: var(--space-4); margin-bottom: var(--space-3); }
.stat { font-size: var(--text-sm); color: var(--text-secondary); }
.snap-meta { display: flex; gap: var(--space-4); margin-bottom: var(--space-4); font-size: var(--text-xs); color: var(--text-tertiary); }
.snap-actions { display: flex; gap: var(--space-2); }
.action-btn { padding: var(--space-1) var(--space-3); border: 1px solid var(--border-default); background: transparent; color: var(--text-secondary); border-radius: var(--radius-sm); font-size: var(--text-xs); cursor: pointer; transition: all var(--transition-fast); }
.action-btn:hover { background: var(--bg-hover); color: var(--text-primary); }
.action-btn.danger { color: var(--color-error); border-color: var(--color-error); }
.action-btn.danger:hover { background: var(--color-error-subtle); }
</style>
