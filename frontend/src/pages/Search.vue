<template>
  <div class="search-page">
    <div class="search-header">
      <div class="search-input-wrapper">
        <svg class="search-icon" width="16" height="16" viewBox="0 0 16 16" fill="none">
          <circle cx="7" cy="7" r="5" stroke="currentColor" stroke-width="1.5"/>
          <path d="M11 11L14 14" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
        <input v-model="query" class="search-input" placeholder="搜索人物、地点、势力、章节..." />
      </div>
      <div class="search-filters">
        <button v-for="f in filters" :key="f.id" class="filter-btn" :class="{ active: activeFilter === f.id }" @click="activeFilter = f.id">{{ f.label }}</button>
      </div>
    </div>
    <div class="search-results">
      <div v-for="result in filteredResults" :key="result.id" class="result-item" @click="navigateTo(result)">
        <span class="result-type">{{ result.type }}</span>
        <span class="result-name">{{ result.name }}</span>
        <span class="result-snippet">{{ result.snippet }}</span>
      </div>
      <div v-if="!filteredResults.length && query" class="empty-state">没有找到匹配的结果</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useRouter } from 'vue-router'
const router = useRouter()
const query = ref('')
const activeFilter = ref('all')
const filters = [
  { id: 'all', label: '全部' },
  { id: 'Character', label: '人物' },
  { id: 'Location', label: '地点' },
  { id: 'Faction', label: '势力' },
  { id: 'Scene', label: '场景' },
]
const allResults = [
  { id: '1', type: 'Character', name: '林凡', snippet: '主角，边境散修，性格坚韧', route: '/project/p1/world/characters' },
  { id: '2', type: 'Character', name: '苏晚晴', snippet: '女主，神秘女子', route: '/project/p1/world/characters' },
  { id: '3', type: 'Location', name: '黑石城', snippet: '天玄大陆北境重镇', route: '/project/p1/world/locations' },
  { id: '4', type: 'Location', name: '地下遗迹', snippet: '远古修士留下的遗迹', route: '/project/p1/world/locations' },
  { id: '5', type: 'Faction', name: '王家', snippet: '黑石城四大家族之首', route: '/project/p1/world/factions' },
  { id: '6', type: 'Scene', name: '场景1：遗迹入口', snippet: '林凡找到地下遗迹的入口', route: '/project/p1/write/scene-1' },
]
const filteredResults = computed(() => {
  let results = allResults
  if (activeFilter.value !== 'all') results = results.filter(r => r.type === activeFilter.value)
  if (query.value) {
    const q = query.value.toLowerCase()
    results = results.filter(r => r.name.toLowerCase().includes(q) || r.snippet.toLowerCase().includes(q))
  }
  return results
})
function navigateTo(result: any) { router.push(result.route) }
</script>

<style scoped>
.search-page { height: 100%; display: flex; flex-direction: column; overflow: hidden; }
.search-header { padding: var(--space-4) var(--space-6); border-bottom: 1px solid var(--border-default); }
.search-input-wrapper { display: flex; align-items: center; gap: var(--space-3); padding: var(--space-3) var(--space-4); background: var(--bg-panel); border: 1px solid var(--border-default); border-radius: var(--radius-md); margin-bottom: var(--space-3); }
.search-icon { color: var(--text-tertiary); flex-shrink: 0; }
.search-input { flex: 1; background: transparent; border: none; outline: none; color: var(--text-primary); font-size: var(--text-lg); }
.search-filters { display: flex; gap: var(--space-2); }
.filter-btn { padding: var(--space-1) var(--space-3); border: 1px solid var(--border-default); background: transparent; color: var(--text-secondary); border-radius: var(--radius-sm); font-size: var(--text-xs); cursor: pointer; }
.filter-btn.active { background: var(--color-primary-subtle); border-color: var(--color-primary); color: var(--color-primary-text); }
.search-results { flex: 1; overflow-y: auto; padding: var(--space-4) var(--space-6); }
.result-item { display: flex; align-items: center; gap: var(--space-3); padding: var(--space-3) var(--space-4); border: 1px solid var(--border-muted); border-radius: var(--radius-md); margin-bottom: var(--space-2); cursor: pointer; transition: all var(--transition-fast); }
.result-item:hover { border-color: var(--border-emphasis); background: var(--bg-hover); }
.result-type { font-size: 10px; padding: 2px 6px; border-radius: 3px; background: var(--bg-panel-secondary); color: var(--text-tertiary); min-width: 60px; text-align: center; }
.result-name { font-weight: 600; min-width: 100px; }
.result-snippet { color: var(--text-secondary); font-size: var(--text-sm); }
.empty-state { padding: var(--space-8); text-align: center; color: var(--text-tertiary); }
</style>
