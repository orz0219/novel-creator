<template>
  <div class="search-page">
    <div class="search-header">
      <div class="search-input-wrapper">
        <svg class="search-icon" width="16" height="16" viewBox="0 0 16 16" fill="none">
          <circle cx="7" cy="7" r="5" stroke="currentColor" stroke-width="1.5"/>
          <path d="M11 11L14 14" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
        <input v-model="query" class="search-input" :placeholder="'搜索人物、地点、势力、物品...'" />
      </div>
      <div class="search-filters">
        <select v-model="typeFilter" class="filter-select">
          <option v-for="f in filters" :key="f.id" :value="f.id">{{ f.label }}</option>
        </select>
      </div>
    </div>

    <div v-if="worldStore.error" class="error-banner">{{ worldStore.error }}</div>

    <div class="search-results">
      <div v-if="worldStore.loading" class="empty-state">加载中...</div>
      <template v-else>
        <div
          v-for="result in results"
          :key="result.id"
          class="result-item"
          @click="navigateTo(result)"
        >
          <span class="result-type">{{ result.entity_type_id }}</span>
          <span class="result-name">{{ result.name }}</span>
          <span class="result-snippet">{{ result.summary || result.description || '' }}</span>
        </div>
        <div v-if="!results.length" class="empty-state">无匹配结果</div>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useRoute, useRouter } from 'vue-router'
import { useWorldStore } from '@/stores/world'
import { useProjectStore } from '@/stores/project'
import { computed, onMounted, ref } from 'vue'
import type { Entity } from '@/types'

const route = useRoute()
const router = useRouter()
const worldStore = useWorldStore()
const projectStore = useProjectStore()

// /search is a top-level route with no :id, so fall back to the active/most-recent project.
const projectId = computed(() => {
  const fromRoute = route.params.id as string | undefined
  return fromRoute || projectStore.currentProject?.id || ''
})
const worldId = computed(() => worldStore.currentWorld?.id ?? '')

const query = ref('')
const typeFilter = ref<string>('all')
const entities = ref<Entity[]>([])

const filters = [
  { id: 'all', label: '全部' },
  { id: 'Character', label: '人物' },
  { id: 'Location', label: '地点' },
  { id: 'Faction', label: '势力' },
  { id: 'Item', label: '物品' },
]

async function ensureProject(): Promise<string> {
  let pid = projectId.value
  if (!pid) {
    if (!projectStore.projects.length) await projectStore.fetchProjects()
    pid = projectStore.currentProject?.id || projectStore.projects[0]?.id || ''
  }
  return pid
}

onMounted(async () => {
  const pid = await ensureProject()
  if (!pid) return
  if (!projectStore.currentProject || projectStore.currentProject.id !== pid) {
    await projectStore.fetchProject(pid)
  }
  if (!worldStore.currentWorld) await worldStore.fetchWorld(pid)
  if (worldId.value) {
    entities.value = await worldStore.fetchEntities(worldId.value)
  }
})

const results = computed<Entity[]>(() => {
  const q = query.value.trim().toLowerCase()
  return entities.value.filter((e) => {
    if (typeFilter.value !== 'all' && e.entity_type_id !== typeFilter.value) return false
    if (!q) return true
    const haystack = [e.name, e.summary ?? '', e.description ?? ''].join(' ').toLowerCase()
    return haystack.includes(q)
  })
})

const typeRouteMap: Record<string, string> = {
  Character: 'characters',
  Location: 'locations',
  Faction: 'factions',
  Item: 'items',
}

function navigateTo(entity: Entity) {
  const segment = typeRouteMap[entity.entity_type_id]
  if (segment) {
    router.push(`/project/${projectId.value}/world/${segment}`)
  }
}
</script>
<style scoped>
.search-page { height: 100%; display: flex; flex-direction: column; overflow: hidden; }
.search-header { padding: var(--space-4) var(--space-6); border-bottom: 1px solid var(--border-default); }
.search-input-wrapper { display: flex; align-items: center; gap: var(--space-3); padding: var(--space-3) var(--space-4); background: var(--bg-panel); border: 1px solid var(--border-default); border-radius: var(--radius-md); margin-bottom: var(--space-3); }
.search-icon { color: var(--text-tertiary); flex-shrink: 0; }
.search-input { flex: 1; background: transparent; border: none; outline: none; color: var(--text-primary); font-size: var(--text-lg); }
.search-filters { display: flex; gap: var(--space-2); }
.filter-select { padding: var(--space-2) var(--space-3); border: 1px solid var(--border-default); background: var(--bg-panel); color: var(--text-secondary); border-radius: var(--radius-sm); font-size: var(--text-sm); cursor: pointer; }
.search-results { flex: 1; overflow-y: auto; padding: var(--space-4) var(--space-6); }
.result-item { display: flex; align-items: center; gap: var(--space-3); padding: var(--space-3) var(--space-4); border: 1px solid var(--border-muted); border-radius: var(--radius-md); margin-bottom: var(--space-2); cursor: pointer; transition: all var(--transition-fast); }
.result-item:hover { border-color: var(--border-emphasis); background: var(--bg-hover); }
.result-type { font-size: 10px; padding: 2px 6px; border-radius: 3px; background: var(--bg-panel-secondary); color: var(--text-tertiary); min-width: 60px; text-align: center; }
.result-name { font-weight: 600; min-width: 100px; }
.result-snippet { color: var(--text-secondary); font-size: var(--text-sm); }
.empty-state { padding: var(--space-8); text-align: center; color: var(--text-tertiary); }
.error-banner { padding: var(--space-3) var(--space-4); background: var(--color-error-subtle); color: var(--color-error); border-radius: var(--radius-sm); margin-bottom: var(--space-4); font-size: var(--text-sm); }
</style>
