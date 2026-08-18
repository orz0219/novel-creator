<template>
  <div class="app-layout">
    <header class="app-header">
      <div class="header-left">
        <router-link to="/" class="logo">
          <span class="logo-icon">笔</span>
          <span class="logo-text">Novel Engine</span>
        </router-link>
      </div>
      <nav class="header-nav">
        <router-link to="/" class="nav-link" :class="{ active: $route.path === '/' }">首页</router-link>
        <router-link v-if="pid" :to="'/project/' + pid" class="nav-link" :class="{ active: isProj && !$route.path.includes('/write') }">项目</router-link>
        <router-link v-if="pid" :to="'/project/' + pid + '/world'" class="nav-link" :class="{ active: $route.path.includes('/world') }">世界</router-link>
        <router-link v-if="pid" :to="'/project/' + pid + '/story'" class="nav-link" :class="{ active: $route.path.includes('/story') }">故事</router-link>
        <router-link v-if="pid" :to="'/project/' + pid + '/write'" class="nav-link" :class="{ active: $route.path.includes('/write') }">创作</router-link>
        <router-link v-if="pid" :to="'/project/' + pid + '/graph'" class="nav-link" :class="{ active: $route.path.includes('/graph') }">图谱</router-link>
      </nav>
      <div class="header-right">
        <router-link to="/search" class="icon-btn" title="搜索">
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none"><circle cx="7" cy="7" r="5" stroke="currentColor" stroke-width="1.5"/><path d="M11 11L14 14" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>
        </router-link>
        <button class="icon-btn" @click="uiStore.openCommandPalette()" title="命令面板 (Ctrl+K)">
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M6 2L2 6L6 10" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/><path d="M10 6L14 10L10 14" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>
        </button>
        <button class="icon-btn" @click="showActivity = !showActivity" title="活动中心">
          <span class="activity-dot" v-if="hasRunningTasks"></span>
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none"><path d="M8 1v6l4 2" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/><circle cx="8" cy="8" r="7" stroke="currentColor" stroke-width="1.5"/></svg>
        </button>
        <router-link to="/settings" class="icon-btn" title="settings">
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none"><circle cx="8" cy="8" r="2" stroke="currentColor" stroke-width="1.5"/><path d="M8 1v2M8 13v2M1 8h2M13 8h2M3.05 3.05l1.41 1.41M11.54 11.54l1.41 1.41M3.05 12.95l1.41-1.41M11.54 4.46l1.41-1.41" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>
        </router-link>
      </div>
    </header>
    <main class="app-main"><RouterView /></main>
    <Transition name="slide">
      <div v-if="showActivity" class="activity-panel">
        <ActivityCenter />
      </div>
    </Transition>

    <CommandPalette v-if="uiStore.commandPaletteOpen" @close="uiStore.closeCommandPalette()" />
    <Toast />
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted } from 'vue'
import { useRoute } from 'vue-router'
import { useUiStore } from '@/stores/ui'
import CommandPalette from '@/components/ui/CommandPalette.vue'
import Toast from '@/components/ui/Toast.vue'
const route = useRoute()
import { ref } from "vue"
const uiStore = useUiStore()
const showActivity = ref(false)
const hasRunningTasks = ref(true)
const pid = computed(() => { const m = route.path.match(/\/project\/([^/]+)/); return m ? m[1] : null })
const isProj = computed(() => route.path.startsWith('/project/' + pid.value))
function handleKeydown(e: KeyboardEvent) {
  if ((e.metaKey || e.ctrlKey) && e.key === 'k') { e.preventDefault(); uiStore.openCommandPalette() }
  if (e.key === 'Escape' && uiStore.commandPaletteOpen) uiStore.closeCommandPalette()
}
onMounted(() => document.addEventListener('keydown', handleKeydown))
onUnmounted(() => document.removeEventListener('keydown', handleKeydown))
</script>

<style scoped>
.app-layout { display: flex; flex-direction: column; height: 100vh; width: 100vw; overflow: hidden; }
.app-header { display: flex; align-items: center; height: var(--header-height); padding: 0 var(--space-4); background: var(--bg-panel); border-bottom: 1px solid var(--border-default); flex-shrink: 0; z-index: var(--z-sticky); }
.header-left { display: flex; align-items: center; gap: var(--space-4); }
.logo { display: flex; align-items: center; gap: var(--space-2); text-decoration: none; color: var(--text-primary); }
.logo-icon { display: flex; align-items: center; justify-content: center; width: 28px; height: 28px; background: var(--color-primary); color: white; border-radius: var(--radius-sm); font-family: var(--font-serif); font-size: var(--text-md); font-weight: 700; }
.logo-text { font-size: var(--text-md); font-weight: 600; letter-spacing: 0.02em; }
.header-nav { display: flex; align-items: center; gap: var(--space-1); margin-left: var(--space-8); }
.nav-link { padding: var(--space-1) var(--space-3); border-radius: var(--radius-sm); font-size: var(--text-sm); color: var(--text-secondary); text-decoration: none; transition: all var(--transition-fast); }
.nav-link:hover { color: var(--text-primary); background: var(--bg-hover); }
.nav-link.active { color: var(--text-primary); background: var(--bg-active); }
.header-right { margin-left: auto; display: flex; align-items: center; gap: var(--space-2); }
.icon-btn { display: flex; align-items: center; justify-content: center; width: 32px; height: 32px; border: none; background: transparent; color: var(--text-secondary); border-radius: var(--radius-sm); cursor: pointer; transition: all var(--transition-fast); text-decoration: none; }
.icon-btn:hover { background: var(--bg-hover); color: var(--text-primary); }
.app-main { flex: 1; overflow: hidden; min-height: 0; }
.activity-panel { position: fixed; top: var(--header-height); right: 0; width: 320px; max-height: 400px; background: var(--bg-panel); border: 1px solid var(--border-default); border-radius: 0 0 0 var(--radius-md); box-shadow: var(--shadow-lg); z-index: var(--z-dropdown); overflow-y: auto; }
.activity-dot { position: absolute; top: 4px; right: 4px; width: 6px; height: 6px; background: var(--color-accent); border-radius: 50%; animation: pulse 1.5s infinite; }
@keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.4; } }
.slide-enter-active, .slide-leave-active { transition: all var(--transition-normal); }
.slide-enter-from, .slide-leave-to { opacity: 0; transform: translateY(-10px); }
</style>