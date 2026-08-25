<template>
  <div class="project-layout">
    <!-- 左侧导航：固定展开 -->
    <aside class="project-rail">
      <div class="rail-head">
        <router-link to="/" class="rail-logo" title="返回首页">
          <span class="logo-seal">笔</span>
          <span class="logo-text">Novel Engine</span>
        </router-link>
      </div>

      <!-- 顶部固定：对话驱动入口与概览 -->
      <div class="rail-pin-top">
        <div class="rail-group">
          <router-link :to="'/project/' + projectId + '/agent'" class="rail-item hero" :title="'创作引导'">
            <Bot class="ri" :size="20" />
            <span class="rl">创作引导</span>
          </router-link>
          <router-link :to="'/project/' + projectId + '/dashboard'" class="rail-item" :title="'概览'">
            <LayoutDashboard class="ri" :size="20" />
            <span class="rl">概览</span>
          </router-link>
        </div>
      </div>

      <!-- 中间可滚动：世界 / 故事 / 工具 -->
      <nav class="rail-scroll">
        <div class="rail-group">
          <div class="rail-group-title">世界</div>
          <router-link :to="'/project/' + projectId + '/world'" class="rail-item" :title="'世界总览'">
            <Globe class="ri" :size="20" /><span class="rl">世界总览</span>
          </router-link>
          <router-link :to="'/project/' + projectId + '/world/characters'" class="rail-item" :title="'人物'">
            <Users class="ri" :size="20" /><span class="rl">人物<span class="rb">{{ worldStore.characters.length }}</span></span>
          </router-link>
          <router-link :to="'/project/' + projectId + '/world/locations'" class="rail-item" :title="'地点'">
            <MapPin class="ri" :size="20" /><span class="rl">地点<span class="rb">{{ worldStore.locations.length }}</span></span>
          </router-link>
          <router-link :to="'/project/' + projectId + '/world/factions'" class="rail-item" :title="'势力'">
            <Swords class="ri" :size="20" /><span class="rl">势力<span class="rb">{{ worldStore.factions.length }}</span></span>
          </router-link>
          <router-link :to="'/project/' + projectId + '/world/items'" class="rail-item" :title="'物品'">
            <Package class="ri" :size="20" /><span class="rl">物品</span>
          </router-link>
          <router-link :to="'/project/' + projectId + '/world/rules'" class="rail-item" :title="'规则'">
            <ScrollText class="ri" :size="20" /><span class="rl">规则</span>
          </router-link>
          <router-link :to="'/project/' + projectId + '/world/relationships'" class="rail-item" :title="'关系'">
            <Link2 class="ri" :size="20" /><span class="rl">关系</span>
          </router-link>
          <router-link :to="'/project/' + projectId + '/world/timeline'" class="rail-item" :title="'时间线'">
            <Calendar class="ri" :size="20" /><span class="rl">时间线</span>
          </router-link>
        </div>

        <div class="rail-group">
          <div class="rail-group-title">故事</div>
          <router-link :to="'/project/' + projectId + '/story'" class="rail-item" :title="'故事结构'">
            <BookOpen class="ri" :size="20" /><span class="rl">故事结构</span>
          </router-link>
          <router-link :to="'/project/' + projectId + '/story/board'" class="rail-item" :title="'看板'">
            <Kanban class="ri" :size="20" /><span class="rl">看板</span>
          </router-link>
          <router-link :to="'/project/' + projectId + '/story/storylines'" class="rail-item" :title="'剧情线'">
            <GitBranch class="ri" :size="20" /><span class="rl">剧情线<span class="rb">{{ storyStore.storylines.length }}</span></span>
          </router-link>
          <router-link :to="'/project/' + projectId + '/story/foreshadows'" class="rail-item" :title="'伏笔'">
            <Wand2 class="ri" :size="20" /><span class="rl">伏笔<span class="rb">{{ storyStore.foreshadows.length }}</span></span>
          </router-link>
        </div>

        <div class="rail-group">
          <div class="rail-group-title">工具</div>
          <router-link :to="'/project/' + projectId + '/graph'" class="rail-item" :title="'关系图谱'">
            <Network class="ri" :size="20" /><span class="rl">关系图谱</span>
          </router-link>
          <router-link :to="'/project/' + projectId + '/proposals'" class="rail-item" :title="'AI 提案'">
            <FileText class="ri" :size="20" /><span class="rl">AI 提案</span>
          </router-link>
          <router-link :to="'/project/' + projectId + '/snapshots'" class="rail-item" :title="'快照'">
            <Camera class="ri" :size="20" /><span class="rl">快照</span>
          </router-link>
          <router-link :to="'/project/' + projectId + '/trace'" class="rail-item" :title="'AI 追溯'">
            <Search class="ri" :size="20" /><span class="rl">AI 追溯</span>
          </router-link>
          <router-link :to="'/project/' + projectId + '/history'" class="rail-item" :title="'历史'">
            <History class="ri" :size="20" /><span class="rl">历史</span>
          </router-link>
        </div>
      </nav>

      <!-- 底部固定：系统 -->
      <div class="rail-pin-bottom">
        <div class="rail-group">
          <div class="rail-group-title">系统</div>
          <router-link to="/search" class="rail-item" :title="'搜索'">
            <Search class="ri" :size="20" /><span class="rl">搜索</span>
          </router-link>
          <button class="rail-item" @click="uiStore.openCommandPalette()" :title="'命令面板'">
            <Command class="ri" :size="20" /><span class="rl">命令面板</span>
          </button>
          <router-link to="/settings" class="rail-item" :title="'设置'">
            <Settings class="ri" :size="20" /><span class="rl">设置</span>
          </router-link>
        </div>
      </div>
    </aside>

    <!-- Main Content -->
    <main class="project-main">
      <RouterView />
    </main>
  </div>
</template>

<script setup lang="ts">
import { computed, watch, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import {
  Bot, Globe, Users, MapPin, Swords, Package, ScrollText, Link2, Calendar,
  BookOpen, Kanban, GitBranch, Wand2, Network, FileText, Camera, Search, History,
  LayoutDashboard, Command, Settings,
} from 'lucide-vue-next'
import { useUiStore } from '@/stores/ui'
import { useWorldStore } from '@/stores/world'
import { useStoryStore } from '@/stores/story'
import { useProjectStore } from '@/stores/project'

const route = useRoute()
const router = useRouter()
const uiStore = useUiStore()
const worldStore = useWorldStore()
const storyStore = useStoryStore()
const projectStore = useProjectStore()

const projectId = computed(() => route.params.id as string)

// Load real data
async function loadData() {
  const pid = projectId.value
  if (!pid) return

  await projectStore.fetchProject(pid)
  await worldStore.fetchWorld(pid)

  if (worldStore.currentWorld) {
    const wid = worldStore.currentWorld.id
    await Promise.all([
      worldStore.fetchCharacters(wid),
      worldStore.fetchLocations(wid),
      worldStore.fetchFactions(wid),
      worldStore.fetchRelations(wid),
    ])
  }

  await Promise.all([
    storyStore.fetchNodes(pid),
    storyStore.fetchStorylines(pid),
    storyStore.fetchForeshadows(pid),
  ])
}

onMounted(() => loadData())
watch(() => route.params.id, () => loadData())

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

/* ---------- 左侧导航（固定展开） ---------- */
.project-rail {
  width: 224px;
  flex-shrink: 0;
  background: var(--bg-panel);
  border-right: 1px solid var(--border-default);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.rail-head {
  display: flex;
  align-items: center;
  height: var(--header-height);
  padding: 0 var(--space-3);
  border-bottom: 1px solid var(--border-muted);
  flex-shrink: 0;
}
.rail-logo {
  display: flex; align-items: center; gap: var(--space-2);
  text-decoration: none;
}
.logo-seal {
  display: flex; align-items: center; justify-content: center;
  width: 30px; height: 30px;
  background: var(--color-primary); color: #fff;
  border-radius: var(--radius-md);
  font-family: var(--font-serif); font-size: var(--text-md); font-weight: 700;
}
.logo-text {
  font-size: var(--text-md);
  font-weight: 600;
  color: var(--text-primary);
  white-space: nowrap;
  letter-spacing: 0.02em;
}

/* 固定区：顶部入口 / 底部系统 */
.rail-pin-top {
  flex-shrink: 0;
  padding: var(--space-2) 0;
  border-bottom: 1px solid var(--border-muted);
}
.rail-pin-bottom {
  flex-shrink: 0;
  padding: var(--space-2) 0;
  border-top: 1px solid var(--border-muted);
}

/* 中间可滚动区 */
.rail-scroll {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  padding: var(--space-2) 0;
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.rail-group { margin-bottom: var(--space-2); }
.rail-group-title {
  padding: var(--space-2) var(--space-3) var(--space-1);
  font-size: var(--text-xs);
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--text-tertiary);
  white-space: nowrap;
}

.rail-item {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  height: 36px;
  padding: 0 var(--space-3);
  margin: 1px var(--space-2);
  border-radius: var(--radius-md);
  color: var(--text-secondary);
  text-decoration: none;
  border: none;
  background: transparent;
  font-family: inherit;
  font-size: var(--text-sm);
  text-align: left;
  transition: all var(--transition-fast);
  cursor: pointer;
  position: relative;
}
.rail-item:hover { background: var(--bg-hover); color: var(--text-primary); }
.rail-item.router-link-active,
.rail-item.router-link-exact-active {
  background: var(--bg-active);
  color: var(--text-primary);
}
.rail-item .ri { flex-shrink: 0; color: var(--text-secondary); }
.rail-item:hover .ri,
.rail-item.router-link-active .ri { color: var(--text-primary); }
.rail-item .rl {
  flex: 1;
  min-width: 0;
  font-size: var(--text-sm);
  white-space: nowrap;
  display: flex; align-items: center; gap: var(--space-2);
}
.rail-item .rb {
  margin-left: auto;
  flex-shrink: 0;
  font-size: var(--text-xs);
  color: var(--text-tertiary);
  background: var(--bg-panel-secondary);
  padding: 1px 6px;
  border-radius: 10px;
}

/* 创作引导：对话驱动主入口，置顶高亮 */
.rail-item.hero {
  color: var(--color-primary-text);
  background: var(--color-primary-subtle);
}
.rail-item.hero .ri { color: var(--color-primary-text); }
.rail-item.hero:hover { background: var(--color-primary-subtle); color: #fff; }
.rail-item.hero:hover .ri { color: #fff; }
.rail-item.hero.router-link-active,
.rail-item.hero.router-link-exact-active {
  background: var(--color-primary);
  color: #fff;
}
.rail-item.hero.router-link-active .ri,
.rail-item.hero.router-link-exact-active .ri { color: #fff; }

.project-main {
  flex: 1;
  overflow: hidden;
  min-width: 0;
}
</style>
