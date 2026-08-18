<template>
  <InspectorPanel entity-type="Location" :entity-name="entity.name" :tabs="tabs" @close="$emit('close')">
    <template #default="{ activeTab }">
      <div v-if="activeTab === 'overview'" class="section">
        <div class="info-grid">
          <div class="info-row"><span class="label">类型</span><span>{{ entity.attributes?.type || '-' }}</span></div>
          <div class="info-row"><span class="label">区域</span><span>{{ entity.attributes?.region || '-' }}</span></div>
          <div class="info-row" v-if="entity.attributes?.population"><span class="label">人口</span><span>{{ entity.attributes.population }}</span></div>
          <div class="info-row" v-if="entity.attributes?.danger_level"><span class="label">危险等级</span><span>{{ entity.attributes.danger_level }}</span></div>
        </div>
        <div class="desc" v-if="entity.description"><div class="section-label">描述</div><p>{{ entity.description }}</p></div>
      </div>
      <div v-if="activeTab === 'entities'" class="section">
        <div class="section-label">相关实体</div>
        <div v-for="e in relatedEntities" :key="e.id" class="related-item">
          <span class="related-type">{{ e.type }}</span>
          <span class="related-name">{{ e.name }}</span>
          <span class="related-rel">{{ e.relation }}</span>
        </div>
      </div>
      <div v-if="activeTab === 'events'" class="section">
        <div class="section-label">相关事件</div>
        <div v-for="ev in relatedEvents" :key="ev.id" class="event-item">
          <span class="event-time">{{ ev.time }}</span>
          <span class="event-name">{{ ev.name }}</span>
        </div>
      </div>
    </template>
  </InspectorPanel>
</template>

<script setup lang="ts">
import InspectorPanel from './InspectorPanel.vue'
import type { Entity } from '@/types'
defineProps<{ entity: Entity }>()
defineEmits(['close'])
const tabs = [
  { id: 'overview', label: '概览' },
  { id: 'entities', label: '实体' },
  { id: 'events', label: '事件' },
  { id: 'history', label: '历史' },
]
const relatedEntities = [
  { id: '1', type: 'Character', name: '林凡', relation: '位于' },
  { id: '2', type: 'Faction', name: '王家', relation: '控制' },
]
const relatedEvents = [
  { id: '1', time: '381-03-10', name: '黑石城大火' },
]
</script>

<style scoped>
.section { margin-bottom: var(--space-4); }
.section-label { font-size: var(--text-xs); font-weight: 600; text-transform: uppercase; color: var(--text-tertiary); margin-bottom: var(--space-2); }
.info-grid { display: flex; flex-direction: column; gap: var(--space-2); }
.info-row { display: flex; justify-content: space-between; font-size: var(--text-sm); }
.label { color: var(--text-tertiary); }
.desc p { font-size: var(--text-sm); color: var(--text-secondary); line-height: var(--leading-relaxed); }
.related-item, .event-item { display: flex; align-items: center; gap: var(--space-3); padding: var(--space-2) 0; border-bottom: 1px solid var(--border-muted); font-size: var(--text-sm); }
.related-type { font-size: 10px; padding: 2px 6px; border-radius: 3px; background: var(--bg-panel-secondary); color: var(--text-tertiary); }
.related-name { font-weight: 500; }
.related-rel { margin-left: auto; color: var(--text-tertiary); font-size: var(--text-xs); }
.event-time { color: var(--text-tertiary); font-size: var(--text-xs); min-width: 80px; }
.event-name { color: var(--text-secondary); }
</style>
