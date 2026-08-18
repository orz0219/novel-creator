<template>
  <div class="project-layout">
    <!-- Left Sidebar -->
    <aside class="project-sidebar" :class="{ collapsed: uiStore.sidebarCollapsed }">
      <div class="sidebar-header">
        <span class="sidebar-title">{{ uiStore.sidebarCollapsed ? '' : '项目导航' }}</span>
        <button class="collapse-btn" @click="uiStore.toggleSidebar()">
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
            <path v-if="!uiStore.sidebarCollapsed" d="M9 3L5 7L9 11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            <path v-else d="M5 3L9 7L5 11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
        </button>
      </div>

      <nav class="sidebar-nav" v-if="!uiStore.sidebarCollapsed">
        <div class="nav-section">
          <div class="nav-section-title">世界</div>
          <router-link :to="'/project/' + projectId + '/world'" class="nav-item">
            <span class="nav-icon">🌍</span>
            <span>世界总览</span>
          </router-link>
          <router-link :to="'/project/' + projectId + '/world/characters'" class="nav-item">
            <span class="nav-icon">👤</span>
            <span>人物</span>
            <span class="nav-badge">{{ worldStore.characters.length }}</span>
          </router-link>
          <router-link :to="'/project/' + projectId + '/world/locations'" class="nav-item">
            <span class="nav-icon">📍</span>
            <span>地点</span>
            <span class="nav-badge">{{ worldStore.locations.length }}</span>
          </router-link>
          <router-link :to="'/project/' + projectId + '/world/factions'" class="nav-item">
            <span class="nav-icon">⚔️</span>
            <span>势力</span>
            <span class="nav-badge">{{ worldStore.factions.length }}</span>
          </router-link>
          <router-link :to="'/project/' + projectId + '/world/items'" class="nav-item">
            <span class="nav-icon">📦</span>
            <span>物品</span>
          </router-link>
          <router-link :to="'/project/' + projectId + '/world/rules'" class="nav-item">
            <span class="nav-icon">📜</span>
            <span>规则</span>
          </router-link>
          <router-link :to="'/project/' + projectId + '/world/relationships'" class="nav-item">
            <span class="nav-icon">🔗</span>
            <span>关系</span>
          </router-link>
          <router-link :to="'/project/' + projectId + '/world/timeline'" class="nav-item">
            <span class="nav-icon">📅</span>
            <span>时间线</span>
          </router-link>
        </div>

        <div class="nav-section">
          <div class="nav-section-title">故事</div>
          <router-link :to="'/project/' + projectId + '/story'" class="nav-item">
            <span class="nav-icon">📖</span>
            <span>故事结构</span>
          </router-link>
          <router-link :to="'/project/' + projectId + '/story/board'" class="nav-item">
            <span class="nav-icon">📋</span>
            <span>看板</span>
          </router-link>
          <router-link :to="'/project/' + projectId + '/story/storylines'" class="nav-item">
            <span class="nav-icon">🧵</span>
            <span>剧情线</span>
            <span class="nav-badge">{{ storyStore.storylines.length }}</span>
          </router-link>
          <router-link :to="'/project/' + projectId + '/story/foreshadows'" class="nav-item">
            <span class="nav-icon">🔮</span>
            <span>伏笔</span>
            <span class="nav-badge">{{ storyStore.foreshadows.length }}</span>
          </router-link>
        </div>

        <div class="nav-section">
          <div class="nav-section-title">工具</div>
          <router-link :to="'/project/' + projectId + '/graph'" class="nav-item">
            <span class="nav-icon">🗺️</span>
            <span>关系图谱</span>
          </router-link>
          <router-link :to="'/project/' + projectId + '/proposals'" class="nav-item">
            <span class="nav-icon">📋</span>
            <span>AI 提案</span>
          </router-link>
          <router-link :to="'/project/' + projectId + '/snapshots'" class="nav-item">
            <span class="nav-icon">📸</span>
            <span>快照</span>
          </router-link>
          <router-link :to="'/project/' + projectId + '/history'" class="nav-item">
            <span class="nav-icon">📜</span>
            <span>历史</span>
          </router-link>
        </div>

        <!-- Story Tree Quick View -->
        <div class="nav-section">
          <div class="nav-section-title">章节</div>
          <div class="story-tree-quick">
            <template v-for="node in storyStore.tree" :key="node.id">
              <div class="tree-node" :class="'level-' + node.node_type.toLowerCase()">
                <span class="tree-toggle" v-if="node.children?.length">▸</span>
                <span class="tree-toggle" v-else>　</span>
                <span class="tree-label truncate">{{ node.title }}</span>
              </div>
              <template v-for="child in node.children" :key="child.id">
                <div class="tree-node" :class="'level-' + child.node_type.toLowerCase()">
                  <span class="tree-toggle" v-if="child.children?.length">▸</span>
                  <span class="tree-toggle" v-else>　</span>
                  <span class="tree-label truncate">{{ child.title }}</span>
                </div>
                <template v-for="grandchild in child.children" :key="grandchild.id">
                  <div
                    class="tree-node clickable"
                    :class="'level-' + grandchild.node_type.toLowerCase()"
                    @click="navigateToNode(grandchild)"
                  >
                    <span class="tree-toggle" v-if="grandchild.children?.length">▸</span>
                    <span class="tree-toggle" v-else>　</span>
                    <span class="tree-label truncate">{{ grandchild.title }}</span>
                    <span class="status-dot" :class="grandchild.status.toLowerCase()"></span>
                  </div>
                </template>
              </template>
            </template>
          </div>
        </div>
      </nav>
    </aside>

    <!-- Main Content -->
    <main class="project-main">
      <RouterView />
    </main>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useUiStore } from '@/stores/ui'
import { useWorldStore } from '@/stores/world'
import { useStoryStore } from '@/stores/story'

const route = useRoute()
const router = useRouter()
const uiStore = useUiStore()
const worldStore = useWorldStore()
const storyStore = useStoryStore()

const projectId = computed(() => route.params.id as string)

// Load mock data
worldStore.loadMockData()
storyStore.loadMockData()

function navigateToNode(node: any) {
  if (node.node_type === 'Scene') {
    router.push('/project/' + projectId.value + '/write/' + node.id)
  }
}
</script>

<style scoped>
.project-layout {
  display: flex;
  height: 100%;
  overflow: hidden;
}

.project-sidebar {
  width: var(--sidebar-width);
  background: var(--bg-panel);
  border-right: 1px solid var(--border-default);
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  overflow: hidden;
  transition: width var(--transition-normal);
}

.project-sidebar.collapsed {
  width: var(--sidebar-collapsed-width);
}

.sidebar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-3) var(--space-3);
  border-bottom: 1px solid var(--border-muted);
  min-height: 40px;
}

.sidebar-title {
  font-size: var(--text-xs);
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--text-tertiary);
}

.collapse-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border: none;
  background: transparent;
  color: var(--text-tertiary);
  border-radius: var(--radius-sm);
  cursor: pointer;
}

.collapse-btn:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.sidebar-nav {
  flex: 1;
  overflow-y: auto;
  padding: var(--space-2) 0;
}

.nav-section {
  margin-bottom: var(--space-2);
}

.nav-section-title {
  padding: var(--space-2) var(--space-3);
  font-size: var(--text-xs);
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--text-tertiary);
}

.nav-item {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-1) var(--space-3);
  margin: 1px var(--space-1);
  border-radius: var(--radius-sm);
  font-size: var(--text-sm);
  color: var(--text-secondary);
  text-decoration: none;
  transition: all var(--transition-fast);
  cursor: pointer;
}

.nav-item:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.nav-item.router-link-active {
  background: var(--bg-active);
  color: var(--text-primary);
}

.nav-icon {
  font-size: var(--text-sm);
  width: 20px;
  text-align: center;
}

.nav-badge {
  margin-left: auto;
  font-size: var(--text-xs);
  color: var(--text-tertiary);
  background: var(--bg-panel-secondary);
  padding: 1px 6px;
  border-radius: 10px;
}

.story-tree-quick {
  padding: 0 var(--space-2);
}

.tree-node {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  padding: 2px var(--space-2);
  border-radius: var(--radius-sm);
  font-size: var(--text-xs);
  color: var(--text-secondary);
}

.tree-node.clickable {
  cursor: pointer;
}

.tree-node.clickable:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.tree-node.level-volume .tree-label { font-weight: 600; color: var(--text-primary); }
.tree-node.level-arc .tree-label { padding-left: 8px; }
.tree-node.level-chapter .tree-label { padding-left: 16px; }
.tree-node.level-scene .tree-label { padding-left: 24px; font-size: 11px; }

.tree-toggle {
  font-size: 10px;
  color: var(--text-tertiary);
  width: 12px;
}

.status-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  margin-left: auto;
  flex-shrink: 0;
}

.status-dot.completed { background: var(--color-success); }
.status-dot.inprogress { background: var(--color-accent); }
.status-dot.planned { background: var(--text-tertiary); }
.status-dot.draft { background: var(--color-warning); }

.project-main {
  flex: 1;
  overflow: hidden;
  min-width: 0;
}
</style>