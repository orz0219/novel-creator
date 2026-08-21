<template>
  <div class="event-log">
    <div class="panel-header">
      <span class="panel-title">事件日志 (Event Log)</span>
      <span class="panel-count">{{ events.length }} 条记录</span>
    </div>

    <div v-if="loading" class="state-box">加载中…</div>
    <div v-else-if="!events.length" class="state-box">暂无事件</div>

    <div v-else class="event-list">
      <div v-for="event in events" :key="event.id" class="event-item">
        <div class="event-time">{{ formatTime(event.created_at) }}</div>
        <div class="event-dot" :class="event.event_type"></div>
        <div class="event-body">
          <div class="event-action">
            <span class="event-type-badge">{{ event.event_type }}</span>
            <span class="event-desc">{{ event.description }}</span>
          </div>
          <div class="event-actor">{{ event.actor }}</div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useRoute } from 'vue-router'
import { historyApi, type EventLogEntry } from '@/api/history'

const route = useRoute()
const projectId = route.params.id as string

const events = ref<EventLogEntry[]>([])
const loading = ref(false)

function formatTime(iso: string): string {
  const d = new Date(iso)
  if (Number.isNaN(d.getTime())) return iso
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${d.getMonth() + 1}月${d.getDate()}日 ${pad(d.getHours())}:${pad(d.getMinutes())}`
}

onMounted(async () => {
  loading.value = true
  try {
    events.value = await historyApi.getEvents(projectId)
  } catch (e: any) {
    events.value = []
  } finally {
    loading.value = false
  }
})
</script>

<style scoped>
.event-log { display: flex; flex-direction: column; }
.panel-header { display: flex; justify-content: space-between; align-items: center; padding: var(--space-3) var(--space-4); border-bottom: 1px solid var(--border-muted); }
.panel-title { font-size: var(--text-sm); font-weight: 600; }
.panel-count { font-size: var(--text-xs); color: var(--text-tertiary); }
.state-box { padding: var(--space-6) var(--space-4); text-align: center; color: var(--text-tertiary); font-size: var(--text-sm); }
.event-list { padding: var(--space-3) var(--space-4); }
.event-item { display: flex; gap: var(--space-3); padding: var(--space-2) 0; position: relative; }
.event-item:not(:last-child)::after {
  content: ''; position: absolute; left: 75px; top: 24px; bottom: -8px;
  width: 1px; background: var(--border-muted);
}
.event-time { font-size: var(--text-xs); color: var(--text-tertiary); min-width: 60px; padding-top: 2px; }
.event-dot { width: 8px; height: 8px; border-radius: 50%; margin-top: 6px; flex-shrink: 0; }
.event-dot.user, .event-dot.ai, .event-dot.system,
.event-dot.create, .event-dot.update, .event-dot.delete, .event-dot.generate,
.event-dot.accept, .event-dot.reject { background: var(--color-accent); }
.event-dot.ai { background: var(--color-primary); }
.event-dot.system { background: var(--text-tertiary); }
.event-body { flex: 1; }
.event-action { font-size: var(--text-sm); font-weight: 500; display: flex; align-items: center; gap: var(--space-2); flex-wrap: wrap; }
.event-type-badge {
  display: inline-flex; align-items: center; padding: 1px 8px;
  border-radius: 10px; font-size: var(--text-xs); font-weight: 500;
  background: var(--bg-panel-secondary); color: var(--text-tertiary);
}
.event-desc { font-size: var(--text-sm); }
.event-actor { font-size: var(--text-xs); color: var(--text-tertiary); margin-top: 1px; }
</style>
