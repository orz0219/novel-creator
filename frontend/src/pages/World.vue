<template>
  <div class="world-page">
    <div class="page-header">
      <h1 class="page-title">世界总览</h1>
    </div>

    <div class="world-stats">
      <div class="stat-card" @click="$router.push('/project/p1/world/characters')">
        <span class="stat-icon">👤</span>
        <span class="stat-value">{{ worldStore.characters.length }}</span>
        <span class="stat-label">人物</span>
      </div>
      <div class="stat-card" @click="$router.push('/project/p1/world/locations')">
        <span class="stat-icon">📍</span>
        <span class="stat-value">{{ worldStore.locations.length }}</span>
        <span class="stat-label">地点</span>
      </div>
      <div class="stat-card" @click="$router.push('/project/p1/world/factions')">
        <span class="stat-icon">⚔️</span>
        <span class="stat-value">{{ worldStore.factions.length }}</span>
        <span class="stat-label">势力</span>
      </div>
      <div class="stat-card" @click="$router.push('/project/p1/world/timeline')">
        <span class="stat-icon">📅</span>
        <span class="stat-value">{{ worldStore.events.length }}</span>
        <span class="stat-label">事件</span>
      </div>
    </div>

    <div class="world-grid">
      <!-- Characters -->
      <div class="panel">
        <div class="panel-header">
          <h3 class="panel-title">人物</h3>
          <router-link to="/project/p1/world/characters" class="panel-link">查看全部 →</router-link>
        </div>
        <div class="entity-list">
          <div v-for="char in worldStore.characters" :key="char.id" class="entity-row">
            <span class="entity-name">{{ char.name }}</span>
            <span class="entity-summary">{{ char.summary }}</span>
            <span class="entity-meta">{{ char.attributes?.identity }}</span>
          </div>
        </div>
      </div>

      <!-- Locations -->
      <div class="panel">
        <div class="panel-header">
          <h3 class="panel-title">地点</h3>
          <router-link to="/project/p1/world/locations" class="panel-link">查看全部 →</router-link>
        </div>
        <div class="entity-list">
          <div v-for="loc in worldStore.locations" :key="loc.id" class="entity-row">
            <span class="entity-name">{{ loc.name }}</span>
            <span class="entity-summary">{{ loc.summary }}</span>
            <span class="entity-meta">{{ loc.attributes?.type }}</span>
          </div>
        </div>
      </div>

      <!-- Factions -->
      <div class="panel">
        <div class="panel-header">
          <h3 class="panel-title">势力</h3>
          <router-link to="/project/p1/world/factions" class="panel-link">查看全部 →</router-link>
        </div>
        <div class="entity-list">
          <div v-for="fac in worldStore.factions" :key="fac.id" class="entity-row">
            <span class="entity-name">{{ fac.name }}</span>
            <span class="entity-summary">{{ fac.summary }}</span>
            <span class="entity-meta">{{ fac.attributes?.leader }}</span>
          </div>
        </div>
      </div>

      <!-- Facts -->
      <div class="panel">
        <div class="panel-header">
          <h3 class="panel-title">世界事实</h3>
        </div>
        <div class="fact-list">
          <div v-for="fact in worldStore.facts" :key="fact.id" class="fact-row">
            <span class="fact-certainty" :class="fact.certainty.toLowerCase()">{{ fact.certainty }}</span>
            <span class="fact-content">{{ fact.content }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useWorldStore } from '@/stores/world'
const worldStore = useWorldStore()
worldStore.loadMockData()
</script>

<style scoped>
.world-page { height: 100%; overflow-y: auto; padding: var(--space-6) var(--space-8); }
.page-header { margin-bottom: var(--space-6); }
.page-title { font-size: var(--text-2xl); font-weight: 700; font-family: var(--font-serif); }

.world-stats {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: var(--space-3);
  margin-bottom: var(--space-6);
}

.stat-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-5);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  background: var(--bg-panel);
  cursor: pointer;
  transition: all var(--transition-fast);
}
.stat-card:hover { border-color: var(--color-primary); background: var(--color-primary-subtle); }
.stat-icon { font-size: 24px; }
.stat-value { font-size: var(--text-2xl); font-weight: 700; }
.stat-label { font-size: var(--text-xs); color: var(--text-tertiary); }

.world-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: var(--space-4);
}

.panel { border: 1px solid var(--border-default); border-radius: var(--radius-md); background: var(--bg-panel); overflow: hidden; }
.panel-header { display: flex; align-items: center; justify-content: space-between; padding: var(--space-3) var(--space-4); border-bottom: 1px solid var(--border-muted); }
.panel-title { font-size: var(--text-md); font-weight: 600; }
.panel-link { font-size: var(--text-xs); color: var(--color-primary-text); text-decoration: none; }

.entity-list { padding: var(--space-2) var(--space-4); }
.entity-row { display: flex; align-items: center; gap: var(--space-3); padding: var(--space-2) 0; border-bottom: 1px solid var(--border-muted); }
.entity-name { font-weight: 500; min-width: 80px; }
.entity-summary { font-size: var(--text-sm); color: var(--text-secondary); flex: 1; }
.entity-meta { font-size: var(--text-xs); color: var(--text-tertiary); }

.fact-list { padding: var(--space-3) var(--space-4); }
.fact-row { display: flex; gap: var(--space-3); padding: var(--space-2) 0; border-bottom: 1px solid var(--border-muted); font-size: var(--text-sm); }
.fact-certainty { font-size: 10px; padding: 2px 6px; border-radius: 3px; flex-shrink: 0; }
.fact-certainty.confirmed { background: var(--color-success-subtle); color: var(--color-success); }
.fact-certainty.likely { background: var(--color-info-subtle); color: var(--color-info); }
.fact-certainty.rumor { background: var(--color-warning-subtle); color: var(--color-warning); }
.fact-content { color: var(--text-secondary); }
</style>
