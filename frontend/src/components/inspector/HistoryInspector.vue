<template>
  <InspectorPanel entity-type="History" :entity-name="entityName" @close="$emit('close')">
    <template #default>
      <div class="section">
        <div class="section-label">版本历史</div>
        <div class="version-list">
          <div v-for="v in versions" :key="v.version" class="version-item">
            <span class="v-num">v{{ v.version }}</span>
            <span class="v-desc">{{ v.description }}</span>
            <span class="v-time">{{ v.time }}</span>
          </div>
        </div>
      </div>
    </template>
  </InspectorPanel>
</template>

<script setup lang="ts">
import InspectorPanel from './InspectorPanel.vue'
defineProps<{
  entityName: string
  versions?: { version: number; description: string; time: string }[]
}>()
defineEmits(['close'])
const versions = [
  { version: 5, description: '最新修改', time: '今天' },
  { version: 4, description: 'AI Proposal #182', time: '昨天' },
  { version: 3, description: '手动修改', time: '3天前' },
  { version: 2, description: '初始创建', time: '1周前' },
]
</script>

<style scoped>
.section { margin-bottom: var(--space-4); }
.section-label { font-size: var(--text-xs); font-weight: 600; text-transform: uppercase; color: var(--text-tertiary); margin-bottom: var(--space-2); }
.version-item { display: flex; gap: var(--space-3); padding: var(--space-2) 0; border-bottom: 1px solid var(--border-muted); font-size: var(--text-sm); }
.v-num { font-family: var(--font-mono); color: var(--text-tertiary); min-width: 30px; }
.v-desc { flex: 1; color: var(--text-secondary); }
.v-time { color: var(--text-tertiary); font-size: var(--text-xs); }
</style>
