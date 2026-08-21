<template>
  <div class="dashboard-page">
    <div v-if="projectStore.loading && !projectStore.currentProject" class="loading-state">
      <span class="loading-text">加载中…</span>
    </div>

    <template v-else>
      <div v-if="projectStore.error" class="error-banner">{{ projectStore.error }}</div>

      <div class="page-header">
        <h1 class="page-title">{{ projectStore.currentProject?.name || '未命名项目' }}</h1>
        <div class="page-subhead">
          <p v-if="projectStore.currentProject?.description" class="page-subtitle">
            {{ projectStore.currentProject.description }}
          </p>
          <StatusBadge
            v-if="projectStore.currentProject?.status"
            :status="(projectStore.currentProject.status || '').toLowerCase()"
            :label="projectStatusLabel[projectStore.currentProject.status] || projectStore.currentProject.status"
          />
        </div>
      </div>

      <div class="dashboard-grid">
        <!-- Stats Cards -->
        <div class="stats-row">
          <router-link class="stat-card" :to="`/project/${projectId}/world/characters`">
            <span class="stat-value">{{ worldStore.characters.length }}</span>
            <span class="stat-label">人物</span>
          </router-link>
          <router-link class="stat-card" :to="`/project/${projectId}/world/locations`">
            <span class="stat-value">{{ worldStore.locations.length }}</span>
            <span class="stat-label">地点</span>
          </router-link>
          <router-link class="stat-card" :to="`/project/${projectId}/world/factions`">
            <span class="stat-value">{{ worldStore.factions.length }}</span>
            <span class="stat-label">势力</span>
          </router-link>
          <router-link class="stat-card" :to="`/project/${projectId}/world/timeline`">
            <span class="stat-value">{{ worldStore.events.length }}</span>
            <span class="stat-label">事件</span>
          </router-link>
          <router-link class="stat-card" :to="`/project/${projectId}/story/storylines`">
            <span class="stat-value">{{ storyStore.storylines.length }}</span>
            <span class="stat-label">剧情线</span>
          </router-link>
          <router-link class="stat-card" :to="`/project/${projectId}/story/foreshadows`">
            <span class="stat-value">{{ storyStore.foreshadows.length }}</span>
            <span class="stat-label">伏笔</span>
          </router-link>
        </div>

        <!-- Characters -->
        <div class="panel">
          <div class="panel-header">
            <h3 class="panel-title">人物</h3>
            <router-link :to="`/project/${projectId}/world/characters`" class="panel-link">查看全部 →</router-link>
          </div>
          <div v-if="worldStore.characters.length" class="entity-list">
            <div v-for="char in worldStore.characters.slice(0, 5)" :key="char.id" class="entity-item">
              <span class="entity-name">{{ char.name }}</span>
              <span v-if="char.summary" class="entity-desc">{{ char.summary }}</span>
            </div>
          </div>
          <div v-else class="panel-empty">暂无人物</div>
        </div>

        <!-- Locations -->
        <div class="panel">
          <div class="panel-header">
            <h3 class="panel-title">地点</h3>
            <router-link :to="`/project/${projectId}/world/locations`" class="panel-link">查看全部 →</router-link>
          </div>
          <div v-if="worldStore.locations.length" class="entity-list">
            <div v-for="loc in worldStore.locations.slice(0, 5)" :key="loc.id" class="entity-item">
              <span class="entity-name">{{ loc.name }}</span>
              <span v-if="loc.summary" class="entity-desc">{{ loc.summary }}</span>
            </div>
          </div>
          <div v-else class="panel-empty">暂无地点</div>
        </div>

        <!-- Storylines -->
        <div class="panel">
          <div class="panel-header">
            <h3 class="panel-title">剧情线</h3>
            <router-link :to="`/project/${projectId}/story/storylines`" class="panel-link">查看全部 →</router-link>
          </div>
          <div v-if="storyStore.storylines.length" class="storyline-list">
            <div v-for="sl in storyStore.storylines.slice(0, 5)" :key="sl.id" class="storyline-item">
              <span class="sl-dot" :class="(sl.status || '').toLowerCase()"></span>
              <span class="sl-name">{{ sl.name }}</span>
              <span class="sl-importance">{{ sl.importance }}</span>
              <StatusBadge :status="(sl.status || '').toLowerCase()" :label="sl.status || ''" />
            </div>
          </div>
          <div v-else class="panel-empty">暂无剧情线</div>
        </div>

        <!-- Foreshadows -->
        <div class="panel">
          <div class="panel-header">
            <h3 class="panel-title">活跃伏笔</h3>
            <router-link :to="`/project/${projectId}/story/foreshadows`" class="panel-link">查看全部 →</router-link>
          </div>
          <div v-if="storyStore.foreshadows.length" class="foreshadow-list">
            <div v-for="fs in storyStore.foreshadows.slice(0, 5)" :key="fs.id" class="foreshadow-item">
              <StatusBadge :status="(fs.status || '').toLowerCase()" :label="fs.status || ''" />
              <span class="fs-name">{{ fs.name }}</span>
              <span class="fs-desc">{{ fs.description }}</span>
            </div>
          </div>
          <div v-else class="panel-empty">暂无伏笔</div>
        </div>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { useProjectStore } from '@/stores/project'
import { useWorldStore } from '@/stores/world'
import { useStoryStore } from '@/stores/story'
import StatusBadge from '@/components/ui/StatusBadge.vue'
import type { ProjectStatus } from '@/types/project'

const route = useRoute()
const projectStore = useProjectStore()
const worldStore = useWorldStore()
const storyStore = useStoryStore()

const projectId = route.params.id as string
const worldId = computed(() => worldStore.currentWorld?.id ?? '')

const projectStatusLabel: Record<ProjectStatus, string> = {
  Concept: '构思中',
  Planning: '规划中',
  Writing: '创作中',
  Paused: '已暂停',
  Completed: '已完成',
  Archived: '已归档',
}

onMounted(async () => {
  await projectStore.fetchProject(projectId)
  await worldStore.fetchWorld(projectId)
  if (worldId.value) {
    await worldStore.fetchCharacters(worldId.value)
    await worldStore.fetchLocations(worldId.value)
    await worldStore.fetchFactions(worldId.value)
    await worldStore.fetchEvents(projectId)
  }
  await storyStore.fetchStorylines(projectId)
  await storyStore.fetchForeshadows(projectId)
})
</script>

<style scoped>
.dashboard-page {
  height: 100%;
  overflow-y: auto;
  padding: var(--space-6) var(--space-8);
}

.loading-state {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: var(--text-tertiary);
}
.loading-text { font-size: var(--text-sm); }

.error-banner {
  padding: var(--space-3) var(--space-4);
  background: var(--color-error-subtle);
  color: var(--color-error);
  border-radius: var(--radius-sm);
  margin-bottom: var(--space-4);
  font-size: var(--text-sm);
}

.page-header {
  margin-bottom: var(--space-8);
}

.page-subhead {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  margin-top: var(--space-2);
  flex-wrap: wrap;
}

.page-title {
  font-size: var(--text-2xl);
  font-weight: 700;
  font-family: var(--font-serif);
}

.page-subtitle {
  font-size: var(--text-sm);
  color: var(--text-secondary);
}

.dashboard-grid {
  display: flex;
  flex-direction: column;
  gap: var(--space-6);
}

.stats-row {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
  gap: var(--space-3);
}

.stat-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: var(--space-5);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  background: var(--bg-panel);
  text-decoration: none;
  transition: border-color 0.15s, background 0.15s;
}
.stat-card:hover {
  border-color: var(--color-primary);
  background: var(--bg-panel-secondary);
}

.stat-value {
  font-size: var(--text-2xl);
  font-weight: 700;
  color: var(--color-primary);
}

.stat-label {
  font-size: var(--text-xs);
  color: var(--text-tertiary);
  margin-top: var(--space-1);
}

.panel {
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  background: var(--bg-panel);
  overflow: hidden;
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-4) var(--space-5);
  border-bottom: 1px solid var(--border-muted);
}

.panel-title {
  font-size: var(--text-md);
  font-weight: 600;
}

.panel-link {
  font-size: var(--text-xs);
  color: var(--color-primary);
  text-decoration: none;
}
.panel-link:hover { color: var(--color-primary-hover); }

.panel-empty {
  padding: var(--space-5);
  color: var(--text-tertiary);
  font-size: var(--text-sm);
}

.entity-list {
  padding: var(--space-3) var(--space-5);
}

.entity-item {
  display: flex;
  align-items: baseline;
  gap: var(--space-3);
  padding: var(--space-2) 0;
  border-bottom: 1px solid var(--border-muted);
  font-size: var(--text-sm);
}
.entity-item:last-child { border-bottom: none; }

.entity-name { font-weight: 500; min-width: 80px; }
.entity-desc { color: var(--text-secondary); }

.storyline-list {
  padding: var(--space-3) var(--space-5);
}

.storyline-item {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-2) 0;
  font-size: var(--text-sm);
}

.sl-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}
.sl-dot.active { background: var(--color-success); }
.sl-dot.planned { background: var(--text-tertiary); }

.sl-name { font-weight: 500; }
.sl-importance { color: var(--text-tertiary); font-size: var(--text-xs); }

.foreshadow-list {
  padding: var(--space-3) var(--space-5);
}

.foreshadow-item {
  display: flex;
  align-items: flex-start;
  gap: var(--space-3);
  padding: var(--space-2) 0;
  font-size: var(--text-sm);
}

.fs-name { font-weight: 500; min-width: 80px; }
.fs-desc { color: var(--text-secondary); }
</style>
