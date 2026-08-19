<template>
  <div class="dashboard-page">
    <div class="page-header">
      <h1 class="page-title">天玄大陆</h1>
      <p class="page-subtitle">一部修仙题材长篇小说 · 创作中</p>
    </div>

    <div class="dashboard-grid">
      <!-- Stats Cards -->
      <div class="stats-row">
        <div class="stat-card">
          <span class="stat-value">{{ worldStore.characters.length }}</span>
          <span class="stat-label">人物</span>
        </div>
        <div class="stat-card">
          <span class="stat-value">{{ worldStore.locations.length }}</span>
          <span class="stat-label">地点</span>
        </div>
        <div class="stat-card">
          <span class="stat-value">{{ worldStore.factions.length }}</span>
          <span class="stat-label">势力</span>
        </div>
        <div class="stat-card">
          <span class="stat-value">{{ worldStore.events.length }}</span>
          <span class="stat-label">事件</span>
        </div>
        <div class="stat-card">
          <span class="stat-value">{{ storyStore.storylines.length }}</span>
          <span class="stat-label">剧情线</span>
        </div>
        <div class="stat-card">
          <span class="stat-value">{{ storyStore.foreshadows.length }}</span>
          <span class="stat-label">伏笔</span>
        </div>
      </div>

      <!-- Current Progress -->
      <div class="panel">
        <div class="panel-header">
          <h3 class="panel-title">当前进度</h3>
          <router-link :to="`/project/${route.params.id}/story`" class="panel-link">查看全部 →</router-link>
        </div>
        <div class="progress-list">
          <div v-for="vol in storyStore.tree" :key="vol.id" class="progress-volume">
            <div class="volume-header">
              <span class="volume-name">{{ vol.title }}</span>
              <span class="volume-status" :class="(vol.status || '').toLowerCase()">{{ vol.status }}</span>
            </div>
            <div v-for="arc in vol.children" :key="arc.id" class="progress-arc">
              <span class="arc-name">{{ arc.title }}</span>
              <div class="arc-chapters">
                <div
                  v-for="ch in arc.children"
                  :key="ch.id"
                  class="chapter-dot"
                  :class="(ch.status || '').toLowerCase()"
                  :title="ch.title"
                >
                  {{ ch.title.replace(/第.*章[：:]\s*/, '').slice(0, 2) }}
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Recent Events -->
      <div class="panel">
        <div class="panel-header">
          <h3 class="panel-title">世界事件</h3>
          <router-link :to="`/project/${route.params.id}/world/timeline`" class="panel-link">查看全部 →</router-link>
        </div>
        <div class="event-list">
          <div v-for="event in worldStore.events" :key="event.id" class="event-item">
            <span class="event-time">{{ event.timestamp }}</span>
            <span class="event-name">{{ event.name }}</span>
            <span class="event-desc">{{ event.description }}</span>
          </div>
        </div>
      </div>

      <!-- Storylines -->
      <div class="panel">
        <div class="panel-header">
          <h3 class="panel-title">剧情线</h3>
          <router-link :to="`/project/${route.params.id}/story/storylines`" class="panel-link">查看全部 →</router-link>
        </div>
        <div class="storyline-list">
          <div v-for="sl in storyStore.storylines" :key="sl.id" class="storyline-item">
            <span class="sl-dot" :class="(sl.status || '').toLowerCase()"></span>
            <span class="sl-name">{{ sl.name }}</span>
            <span class="sl-importance">{{ sl.importance }}</span>
            <span class="sl-status">{{ sl.status }}</span>
          </div>
        </div>
      </div>

      <!-- Foreshadows -->
      <div class="panel">
        <div class="panel-header">
          <h3 class="panel-title">活跃伏笔</h3>
          <router-link :to="`/project/${route.params.id}/story/foreshadows`" class="panel-link">查看全部 →</router-link>
        </div>
        <div class="foreshadow-list">
          <div v-for="fs in storyStore.foreshadows" :key="fs.id" class="foreshadow-item">
            <span class="fs-badge" :class="(fs.status || '').toLowerCase()">{{ fs.status }}</span>
            <span class="fs-name">{{ fs.name }}</span>
            <span class="fs-desc">{{ fs.description }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useRoute } from "vue-router"
const route = useRoute()
import { useWorldStore } from '@/stores/world'
import { useStoryStore } from '@/stores/story'

const worldStore = useWorldStore()
const storyStore = useStoryStore()

</script>

<style scoped>
.dashboard-page {
  height: 100%;
  overflow-y: auto;
  padding: var(--space-6) var(--space-8);
}

.page-header {
  margin-bottom: var(--space-8);
}

.page-title {
  font-size: var(--text-2xl);
  font-weight: 700;
  font-family: var(--font-serif);
}

.page-subtitle {
  font-size: var(--text-sm);
  color: var(--text-secondary);
  margin-top: var(--space-1);
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
}

.stat-value {
  font-size: var(--text-2xl);
  font-weight: 700;
  color: var(--color-primary-text);
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
  color: var(--color-primary-text);
  text-decoration: none;
}
.panel-link:hover { color: var(--color-primary-hover); }

.progress-list {
  padding: var(--space-4) var(--space-5);
}

.progress-volume {
  margin-bottom: var(--space-4);
}

.volume-header {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  margin-bottom: var(--space-2);
}

.volume-name {
  font-size: var(--text-sm);
  font-weight: 600;
}

.volume-status {
  font-size: var(--text-xs);
  padding: 2px 8px;
  border-radius: 10px;
}
.volume-status.inprogress { background: var(--color-accent-subtle); color: var(--color-accent); }

.progress-arc {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-1) 0 var(--space-1) var(--space-4);
}

.arc-name {
  font-size: var(--text-sm);
  color: var(--text-secondary);
  min-width: 100px;
}

.arc-chapters {
  display: flex;
  gap: var(--space-2);
}

.chapter-dot {
  width: 32px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-sm);
  font-size: 10px;
  font-weight: 500;
  cursor: default;
}

.chapter-dot.completed { background: var(--color-success-subtle); color: var(--color-success); }
.chapter-dot.inprogress { background: var(--color-accent-subtle); color: var(--color-accent); }
.chapter-dot.planned { background: var(--bg-panel-secondary); color: var(--text-tertiary); }
.chapter-dot.draft { background: var(--color-warning-subtle); color: var(--color-warning); }

.event-list {
  padding: var(--space-3) var(--space-5);
}

.event-item {
  display: flex;
  gap: var(--space-4);
  padding: var(--space-2) 0;
  border-bottom: 1px solid var(--border-muted);
  font-size: var(--text-sm);
}

.event-time {
  color: var(--text-tertiary);
  min-width: 120px;
  font-size: var(--text-xs);
}

.event-name {
  font-weight: 500;
  min-width: 100px;
}

.event-desc {
  color: var(--text-secondary);
}

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
}
.sl-dot.active { background: var(--color-success); }
.sl-dot.planned { background: var(--text-tertiary); }

.sl-name { font-weight: 500; }
.sl-importance { color: var(--text-tertiary); font-size: var(--text-xs); }
.sl-status { margin-left: auto; color: var(--text-tertiary); font-size: var(--text-xs); }

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

.fs-badge {
  font-size: 10px;
  padding: 2px 6px;
  border-radius: 3px;
  flex-shrink: 0;
}
.fs-badge.introduced { background: var(--color-info-subtle); color: var(--color-info); }
.fs-badge.active { background: var(--color-warning-subtle); color: var(--color-warning); }

.fs-name { font-weight: 500; min-width: 80px; }
.fs-desc { color: var(--text-secondary); }
</style>
