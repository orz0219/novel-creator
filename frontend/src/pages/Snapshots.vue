<template>
  <div class="snapshots-page">
    <div class="page-header">
      <h1 class="page-title">世界快照</h1>
      <button class="btn-primary" @click="handleCreate">+ 创建快照</button>
    </div>
    <div v-if="snapshots.length" class="snapshot-list">
      <div v-for="snap in snapshots" :key="snap.id" class="snapshot-card">
        <div class="snap-header">
          <span class="snap-id">#{{ snap.id.slice(0, 8) }}</span>
          <span class="snap-name">{{ snap.name }}</span>
          <span class="snap-time">{{ formatDate(snap.created_at) }}</span>
        </div>
        <div class="snap-stats">
          <span class="stat">👤 {{ snap.known_characters_count }} 人物</span>
          <span class="stat">📍 {{ snap.known_locations_count }} 地点</span>
          <span class="stat">🧵 {{ snap.active_threads_count }} 剧情线</span>
          <span class="stat">🔮 {{ snap.unresolved_foreshadows_count }} 伏笔</span>
        </div>
        <div class="snap-meta">
          <span v-if="snap.story_time">时间线: {{ snap.story_time }}</span>
          <span v-if="snap.progress">进度: {{ snap.progress }}</span>
        </div>
        <div class="snap-actions">
          <button class="action-btn danger" @click="handleDelete(snap.id)">删除</button>
        </div>
      </div>
    </div>
    <div v-else class="empty-state">
      <p>暂无快照</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { snapshotsApi, type Snapshot } from '@/api/snapshots'

const route = useRoute()
const snapshots = ref<Snapshot[]>([])

async function loadSnapshots() {
  const projectId = route.params.id as string
  try {
    snapshots.value = await snapshotsApi.list(projectId)
  } catch {
    snapshots.value = []
  }
}

onMounted(loadSnapshots)

function formatDate(dateStr: string) {
  try {
    const d = new Date(dateStr)
    return `${d.getMonth() + 1}月${d.getDate()}日 ${d.getHours()}:${String(d.getMinutes()).padStart(2, '0')}`
  } catch {
    return dateStr
  }
}

async function handleCreate() {
  const projectId = route.params.id as string
  await snapshotsApi.create(projectId, {
    name: '手动快照',
  })
  await loadSnapshots()
}

async function handleDelete(id: string) {
  await snapshotsApi.delete(id)
  await loadSnapshots()
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
.snapshot-card:hover { border-color: var(--border-emphasis); }
.snap-header { display: flex; align-items: center; gap: var(--space-3); margin-bottom: var(--space-3); }
.snap-id { font-family: var(--font-mono); font-size: var(--text-sm); color: var(--text-tertiary); }
.snap-name { font-size: var(--text-md); font-weight: 600; }
.snap-time { margin-left: auto; font-size: var(--text-xs); color: var(--text-tertiary); }
.snap-stats { display: flex; gap: var(--space-4); margin-bottom: var(--space-3); }
.stat { font-size: var(--text-sm); color: var(--text-secondary); }
.snap-meta { display: flex; gap: var(--space-4); margin-bottom: var(--space-4); font-size: var(--text-xs); color: var(--text-tertiary); }
.snap-actions { display: flex; gap: var(--space-2); }
.action-btn { padding: var(--space-1) var(--space-3); border: 1px solid var(--border-default); background: transparent; color: var(--text-secondary); border-radius: var(--radius-sm); font-size: var(--text-xs); cursor: pointer; transition: all var(--transition-fast); }
.action-btn:hover { background: var(--bg-hover); color: var(--text-primary); }
.action-btn.danger { color: var(--color-error); border-color: var(--color-error); }
.action-btn.danger:hover { background: var(--color-error-subtle); }
.empty-state { padding: var(--space-12); text-align: center; color: var(--text-tertiary); }
</style>
