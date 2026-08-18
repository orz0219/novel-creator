<template>
  <InspectorPanel entity-type="Context" entity-name="当前上下文" @close="$emit('close')">
    <template #default>
      <div class="section">
        <div class="section-label">上下文统计</div>
        <div class="info-grid">
          <div class="info-row"><span class="label">实体数</span><span>{{ entities.length }}</span></div>
          <div class="info-row"><span class="label">Token 数</span><span>{{ totalTokens.toLocaleString() }}</span></div>
        </div>
      </div>
      <div class="section">
        <div class="section-label">已钉住</div>
        <div v-for="e in pinned" :key="e.entity_id" class="ctx-item">{{ e.entity_name }}</div>
        <div v-if="!pinned.length" class="empty">无</div>
      </div>
      <div class="section">
        <div class="section-label">已排除</div>
        <div v-for="e in excluded" :key="e.entity_id" class="ctx-item">{{ e.entity_name }}</div>
        <div v-if="!excluded.length" class="empty">无</div>
      </div>
    </template>
  </InspectorPanel>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import InspectorPanel from './InspectorPanel.vue'
import type { ContextEntity } from '@/types'
const props = defineProps<{ entities: ContextEntity[]; totalTokens: number }>()
defineEmits(['close'])
const pinned = computed(() => props.entities.filter(e => e.policy === 'Pinned'))
const excluded = computed(() => props.entities.filter(e => e.policy === 'Excluded'))
</script>

<style scoped>
.section { margin-bottom: var(--space-4); }
.section-label { font-size: var(--text-xs); font-weight: 600; text-transform: uppercase; color: var(--text-tertiary); margin-bottom: var(--space-2); }
.info-grid { display: flex; flex-direction: column; gap: var(--space-2); }
.info-row { display: flex; justify-content: space-between; font-size: var(--text-sm); }
.label { color: var(--text-tertiary); }
.ctx-item { font-size: var(--text-sm); color: var(--text-secondary); padding: var(--space-1) 0; }
.empty { font-size: var(--text-xs); color: var(--text-tertiary); }
</style>
