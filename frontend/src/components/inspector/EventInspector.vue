<template>
  <InspectorPanel entity-type="Event" :entity-name="event.name" @close="$emit('close')">
    <template #default>
      <div class="section">
        <div class="info-grid">
          <div class="info-row"><span class="label">类型</span><span>{{ event.event_type || '-' }}</span></div>
          <div class="info-row"><span class="label">时间</span><span>{{ event.timestamp || '-' }}</span></div>
        </div>
        <div class="desc" v-if="event.description"><div class="section-label">描述</div><p>{{ event.description }}</p></div>
      </div>
    </template>
  </InspectorPanel>
</template>

<script setup lang="ts">
import InspectorPanel from './InspectorPanel.vue'
import type { Event } from '@/types'
defineProps<{ event: Event }>()
defineEmits(['close'])
</script>

<style scoped>
.section { margin-bottom: var(--space-4); }
.section-label { font-size: var(--text-xs); font-weight: 600; text-transform: uppercase; color: var(--text-tertiary); margin-bottom: var(--space-2); }
.info-grid { display: flex; flex-direction: column; gap: var(--space-2); }
.info-row { display: flex; justify-content: space-between; font-size: var(--text-sm); }
.label { color: var(--text-tertiary); }
.desc p { font-size: var(--text-sm); color: var(--text-secondary); }
</style>
