<template>
  <div class="activity-center">
    <div class="panel-header">
      <span class="panel-title">活动中心</span>
      <button class="clear-btn" @click="activities = []">清除</button>
    </div>
    <div class="activity-list">
      <div v-for="activity in activities" :key="activity.id" class="activity-item" :class="activity.status">
        <span class="activity-icon">{{ statusIcons[activity.status] }}</span>
        <div class="activity-body">
          <span class="activity-text">{{ activity.text }}</span>
          <span class="activity-time">{{ activity.time }}</span>
        </div>
        <button v-if="activity.status === 'running'" class="cancel-btn" @click="cancelActivity(activity.id)">取消</button>
      </div>
      <div v-if="!activities.length" class="empty-state">暂无活动</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
const statusIcons: Record<string, string> = { completed: '✓', running: '●', failed: '✗', warning: '⚠' }
const activities = ref([
  { id: '1', text: '场景生成完成', status: 'completed', time: '2分钟前' },
  { id: '2', text: '验证完成 - 1 个警告', status: 'warning', time: '1分钟前' },
  { id: '3', text: '正在构建 Context...', status: 'running', time: '进行中' },
])
function cancelActivity(id: string) { activities.value = activities.value.filter(a => a.id !== id) }
</script>

<style scoped>
.activity-center { display: flex; flex-direction: column; }
.panel-header { display: flex; justify-content: space-between; align-items: center; padding: var(--space-3) var(--space-4); border-bottom: 1px solid var(--border-muted); }
.panel-title { font-size: var(--text-sm); font-weight: 600; }
.clear-btn { border: none; background: transparent; color: var(--text-tertiary); font-size: var(--text-xs); cursor: pointer; }
.activity-list { padding: var(--space-2); }
.activity-item { display: flex; align-items: center; gap: var(--space-3); padding: var(--space-2) var(--space-3); border-radius: var(--radius-sm); margin-bottom: var(--space-1); }
.activity-item:hover { background: var(--bg-hover); }
.activity-icon { font-size: var(--text-sm); width: 16px; text-align: center; }
.activity-item.completed .activity-icon { color: var(--color-success); }
.activity-item.running .activity-icon { color: var(--color-accent); animation: pulse 1.5s infinite; }
.activity-item.failed .activity-icon { color: var(--color-error); }
.activity-item.warning .activity-icon { color: var(--color-warning); }
.activity-body { flex: 1; }
.activity-text { font-size: var(--text-sm); display: block; }
.activity-time { font-size: var(--text-xs); color: var(--text-tertiary); }
.cancel-btn { border: 1px solid var(--border-default); background: transparent; color: var(--text-secondary); border-radius: var(--radius-sm); font-size: var(--text-xs); padding: 2px 8px; cursor: pointer; }
.empty-state { padding: var(--space-6); text-align: center; color: var(--text-tertiary); font-size: var(--text-sm); }
@keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.4; } }
</style>
