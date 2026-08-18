<template>
  <InspectorPanel entity-type="Scene" :entity-name="scene.title" :tabs="tabs" @close="$emit('close')">
    <template #default="{ activeTab }">
      <div v-if="activeTab === 'overview'" class="section">
        <div class="info-grid">
          <div class="info-row"><span class="label">状态</span><span>{{ scene.status }}</span></div>
          <div class="info-row" v-if="scene.attributes?.time"><span class="label">时间</span><span>{{ scene.attributes.time }}</span></div>
          <div class="info-row" v-if="scene.attributes?.objective"><span class="label">目标</span><span>{{ scene.attributes.objective }}</span></div>
          <div class="info-row" v-if="scene.attributes?.conflict"><span class="label">冲突</span><span>{{ scene.attributes.conflict }}</span></div>
        </div>
      </div>
      <div v-if="activeTab === 'characters'" class="section">
        <div class="section-label">出场角色</div>
        <div v-for="cid in scene.attributes?.characters_present || []" :key="cid" class="char-item">
          <span class="char-dot">👤</span>
          <span>{{ cid }}</span>
        </div>
      </div>
    </template>
  </InspectorPanel>
</template>

<script setup lang="ts">
import InspectorPanel from './InspectorPanel.vue'
import type { NarrativeNode } from '@/types'
defineProps<{ scene: NarrativeNode }>()
defineEmits(['close'])
const tabs = [{ id: 'overview', label: '概览' }, { id: 'characters', label: '角色' }, { id: 'history', label: '历史' }]
</script>

<style scoped>
.section { margin-bottom: var(--space-4); }
.section-label { font-size: var(--text-xs); font-weight: 600; text-transform: uppercase; color: var(--text-tertiary); margin-bottom: var(--space-2); }
.info-grid { display: flex; flex-direction: column; gap: var(--space-2); }
.info-row { display: flex; justify-content: space-between; font-size: var(--text-sm); }
.label { color: var(--text-tertiary); }
.char-item { display: flex; align-items: center; gap: var(--space-2); padding: var(--space-1) 0; font-size: var(--text-sm); }
.char-dot { font-size: 12px; }
</style>
