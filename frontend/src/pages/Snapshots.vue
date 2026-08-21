<template>
  <div class="snapshots-page">
    <div class="page-header">
      <h1 class="page-title">世界快照</h1>
      <button class="btn-primary" @click="handleCreate">+ 创建快照</button>
    </div>

    <div v-if="loading" class="empty-state">
      <span class="empty-text">加载中…</span>
    </div>

    <div v-else-if="snapshots.length" class="snapshot-list">
      <div v-for="snap in snapshots" :key="snap.id" class="snapshot-card">
        <div class="snap-header">
          <span class="snap-id">#{{ snap.id.slice(0, 8) }}</span>
          <span class="snap-name">{{ snap.name }}</span>
          <span class="snap-time">{{ formatDate(snap.created_at) }}</span>
        </div>

        <div class="snap-field" v-if="snap.story_time">
          <span class="field-label">时间线</span>
          <span class="field-value">{{ snap.story_time }}</span>
        </div>

        <div class="snap-field" v-if="snap.current_location">
          <span class="field-label">当前位置</span>
          <span class="field-value">{{ snap.current_location }}</span>
        </div>

        <div class="snap-field snap-summary" v-if="snap.world_summary">
          <span class="field-label">世界摘要</span>
          <p class="field-value">{{ snap.world_summary }}</p>
        </div>

        <div class="snap-stats">
          <span class="stat">🧵 活跃线程 {{ snap.active_threads_count }}</span>
          <span class="stat">🔮 未解伏笔 {{ snap.unresolved_foreshadows_count }}</span>
          <span class="stat">👤 已知人物 {{ snap.known_characters_count }}</span>
          <span class="stat">📍 已知地点 {{ snap.known_locations_count }}</span>
          <span class="stat" v-if="snap.progress">📈 进度 {{ snap.progress }}</span>
          <span class="stat">🕒 创建时间 {{ formatDate(snap.created_at) }}</span>
        </div>

        <div class="snap-actions">
          <button class="action-btn danger" @click="handleDelete(snap.id)">删除</button>
        </div>
      </div>
    </div>

    <div v-else class="empty-state">
      <span class="empty-icon">🌍</span>
      <span class="empty-text">暂无快照</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRoute } from 'vue-router'
import { useProjectStore } from '@/stores/project'
import { snapshotsApi, type Snapshot } from '@/api/snapshots'

const route = useRoute()
const projectStore = useProjectStore()
const projectId = (route.params.id as string) || projectStore.currentProject?.id || ''

const snapshots = ref<Snapshot[]>([])
const loading = ref(false)

onMounted(async () => {
  loading.value = true
  try {
    snapshots.value = await snapshotsApi.list(projectId).catch(() => [])
  } finally {
    loading.value = false
  }
})

function formatDate(dateStr: string) {
  try {
    const d = new Date(dateStr)
    return `${d.getMonth() + 1}月${d.getDate()}日 ${d.getHours()}:${String(d.getMinutes()).padStart(2, '0')}`
  } catch {
    return dateStr
  }
}

async function handleCreate() {
  await snapshotsApi.create(projectId, { name: '手动快照' })
  snapshots.value = await snapshotsApi.list(projectId).catch(() => [])
}

async function handleDelete(id: string) {
  await snapshotsApi.delete(id)
  snapshots.value = await snapshotsApi.list(projectId).catch(() => [])
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
.snap-field { display: flex; gap: var(--space-3); margin-bottom: var(--space-2); font-size: var(--text-sm); line-height: 1.6; }
.field-label { flex: 0 0 64px; color: var(--text-tertiary); }
.field-value { flex: 1; color: var(--text-primary); white-space: pre-wrap; margin: 0; }
.snap-summary { flex-direction: column; gap: var(--space-1); }
.snap-summary .field-label { flex: none; }
.snap-stats { display: flex; flex-wrap: wrap; gap: var(--space-4); margin-top: var(--space-3); margin-bottom: var(--space-3); }
.stat { font-size: var(--text-sm); color: var(--text-secondary); }
.snap-actions { display: flex; gap: var(--space-2); }
.action-btn { padding: var(--space-1) var(--space-3); border: 1px solid var(--border-default); background: transparent; color: var(--text-secondary); border-radius: var(--radius-sm); font-size: var(--text-xs); cursor: pointer; transition: all var(--transition-fast); }
.action-btn:hover { background: var(--bg-hover); color: var(--text-primary); }
.action-btn.danger { color: var(--color-error); border-color: var(--color-error); }
.action-btn.danger:hover { background: var(--color-error-subtle); }
.empty-state { display: flex; flex-direction: column; align-items: center; justify-content: center; padding: var(--space-16); color: var(--text-tertiary); }
.empty-icon { font-size: 48px; margin-bottom: var(--space-4); }
.empty-text { font-size: var(--text-sm); }
</style>
