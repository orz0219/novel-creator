<template>
  <div class="command-overlay" @click.self="$emit('close')">
    <div class="command-palette">
      <div class="command-input-wrapper">
        <svg class="search-icon" width="16" height="16" viewBox="0 0 16 16" fill="none"><circle cx="7" cy="7" r="5" stroke="currentColor" stroke-width="1.5"/><path d="M11 11L14 14" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>
        <input ref="inputRef" v-model="query" class="command-input" placeholder="搜索命令..." @keydown.escape="$emit('close')" @keydown.enter="executeSelected" @keydown.up.prevent="selectedIndex = Math.max(0, selectedIndex - 1)" @keydown.down.prevent="selectedIndex = Math.min(filtered.length - 1, selectedIndex + 1)" />
        <span class="command-hint">ESC</span>
      </div>
      <div class="command-list" v-if="filtered.length">
        <div v-for="(cmd, i) in filtered" :key="cmd.id" class="command-item" :class="{ selected: i === selectedIndex }" @click="cmd.action(); $emit('close')" @mouseenter="selectedIndex = i">
          <span class="cmd-icon">{{ cmd.icon }}</span>
          <span class="cmd-label">{{ cmd.label }}</span>
          <span class="cmd-cat">{{ cmd.cat }}</span>
        </div>
      </div>
      <div class="command-empty" v-else-if="query">没有找到匹配的命令</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
defineEmits(['close'])
const router = useRouter()
const inputRef = ref<HTMLInputElement>()
const query = ref('')
const selectedIndex = ref(0)
const cmds = [
  { id: 'home', icon: '🏠', label: '返回首页', cat: '导航', action: () => router.push('/') },
  { id: 'proj', icon: '📁', label: '项目仪表盘', cat: '导航', action: () => router.push('/project/p1') },
  { id: 'world', icon: '🌍', label: '世界总览', cat: '世界', action: () => router.push('/project/p1/world') },
  { id: 'chars', icon: '👤', label: '人物列表', cat: '世界', action: () => router.push('/project/p1/world/characters') },
  { id: 'locs', icon: '📍', label: '地点列表', cat: '世界', action: () => router.push('/project/p1/world/locations') },
  { id: 'facs', icon: '⚔️', label: '势力列表', cat: '世界', action: () => router.push('/project/p1/world/factions') },
  { id: 'tl', icon: '📅', label: '时间线', cat: '世界', action: () => router.push('/project/p1/world/timeline') },
  { id: 'story', icon: '📖', label: '故事结构', cat: '故事', action: () => router.push('/project/p1/story') },
  { id: 'sl', icon: '🧵', label: '剧情线', cat: '故事', action: () => router.push('/project/p1/story/storylines') },
  { id: 'fs', icon: '🔮', label: '伏笔管理', cat: '故事', action: () => router.push('/project/p1/story/foreshadows') },
  { id: 'write', icon: '✍️', label: '进入写作', cat: '创作', action: () => router.push('/project/p1/write/scene-1') },
  { id: 'graph', icon: '🗺️', label: '关系图谱', cat: '工具', action: () => router.push('/project/p1/graph') },
  { id: 'prop', icon: '📋', label: 'AI 提案', cat: 'AI', action: () => router.push('/project/p1/proposals') },
  { id: 'hist', icon: '📜', label: '历史记录', cat: '工具', action: () => router.push('/project/p1/history') },
  { id: 'search', icon: '🔍', label: '全局搜索', cat: '工具', action: () => router.push('/search') },
  { id: 'settings', icon: '⚙️', label: '设置', cat: '系统', action: () => router.push('/settings') },
]
const filtered = computed(() => {
  if (!query.value) return cmds
  const q = query.value.toLowerCase()
  return cmds.filter(c => c.label.toLowerCase().includes(q) || c.cat.toLowerCase().includes(q))
})
function executeSelected() { if (filtered.value[selectedIndex.value]) { filtered.value[selectedIndex.value].action() } }
onMounted(() => inputRef.value?.focus())
</script>

<style scoped>
.command-overlay { position: fixed; inset: 0; background: var(--bg-overlay); display: flex; align-items: flex-start; justify-content: center; padding-top: 20vh; z-index: var(--z-modal); }
.command-palette { width: 520px; max-height: 400px; background: var(--bg-panel); border: 1px solid var(--border-emphasis); border-radius: var(--radius-lg); box-shadow: var(--shadow-xl); overflow: hidden; display: flex; flex-direction: column; }
.command-input-wrapper { display: flex; align-items: center; gap: var(--space-2); padding: var(--space-3) var(--space-4); border-bottom: 1px solid var(--border-default); }
.search-icon { color: var(--text-tertiary); flex-shrink: 0; }
.command-input { flex: 1; background: transparent; border: none; outline: none; color: var(--text-primary); font-size: var(--text-md); }
.command-input::placeholder { color: var(--text-tertiary); }
.command-hint { font-size: var(--text-xs); color: var(--text-tertiary); padding: 2px 6px; border: 1px solid var(--border-default); border-radius: var(--radius-sm); }
.command-list { overflow-y: auto; padding: var(--space-2); }
.command-item { display: flex; align-items: center; gap: var(--space-3); padding: var(--space-2) var(--space-3); border-radius: var(--radius-sm); cursor: pointer; transition: background var(--transition-fast); }
.command-item.selected, .command-item:hover { background: var(--bg-hover); }
.cmd-icon { font-size: var(--text-sm); width: 24px; text-align: center; }
.cmd-label { flex: 1; font-size: var(--text-sm); color: var(--text-primary); }
.cmd-cat { font-size: var(--text-xs); color: var(--text-tertiary); padding: 1px 6px; background: var(--bg-panel-secondary); border-radius: 3px; }
.command-empty { padding: var(--space-6); text-align: center; color: var(--text-tertiary); font-size: var(--text-sm); }
</style>
