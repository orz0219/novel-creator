<template>
  <div class="event-log">
    <div class="panel-header">
      <span class="panel-title">事件日志 (Event Log)</span>
      <span class="panel-count">{{ events.length }} 条记录</span>
    </div>
    <div class="event-list">
      <div v-for="event in events" :key="event.id" class="event-item">
        <div class="event-time">{{ event.time }}</div>
        <div class="event-dot" :class="event.type"></div>
        <div class="event-body">
          <div class="event-action">{{ event.action }}</div>
          <div class="event-detail" v-if="event.detail">{{ event.detail }}</div>
          <div class="event-actor">{{ event.actor }}</div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
const events = [
  { id: '1', time: '3月12日 14:30', type: 'user', action: '创建角色', detail: '林凡 - 边境散修', actor: '用户' },
  { id: '2', time: '3月12日 14:35', type: 'user', action: '创建地点', detail: '黑石城 - 北境重镇', actor: '用户' },
  { id: '3', time: '3月12日 15:00', type: 'ai', action: 'AI 生成地点', detail: '地下遗迹', actor: 'AI (mimo-v2.5)' },
  { id: '4', time: '3月12日 15:05', type: 'user', action: '接受 Proposal', detail: '#182 - 新增地下遗迹', actor: '用户' },
  { id: '5', time: '3月12日 16:00', type: 'ai', action: 'AI 生成场景', detail: '场景1：遗迹入口', actor: 'AI (mimo-v2.5)' },
  { id: '6', time: '3月12日 16:10', type: 'user', action: '编辑场景', detail: '场景1：遗迹入口 - 修改开头', actor: '用户' },
  { id: '7', time: '3月12日 17:00', type: 'system', action: '自动验证', detail: '1 warning - 时间线一致性', actor: '系统' },
  { id: '8', time: '3月12日 18:00', type: 'user', action: '钉住 Context', detail: '古井 - 伏笔地点', actor: '用户' },
]
</script>

<style scoped>
.event-log { display: flex; flex-direction: column; }
.panel-header { display: flex; justify-content: space-between; align-items: center; padding: var(--space-3) var(--space-4); border-bottom: 1px solid var(--border-muted); }
.panel-title { font-size: var(--text-sm); font-weight: 600; }
.panel-count { font-size: var(--text-xs); color: var(--text-tertiary); }
.event-list { padding: var(--space-3) var(--space-4); }
.event-item { display: flex; gap: var(--space-3); padding: var(--space-2) 0; position: relative; }
.event-item:not(:last-child)::after {
  content: ''; position: absolute; left: 75px; top: 24px; bottom: -8px;
  width: 1px; background: var(--border-muted);
}
.event-time { font-size: var(--text-xs); color: var(--text-tertiary); min-width: 60px; padding-top: 2px; }
.event-dot { width: 8px; height: 8px; border-radius: 50%; margin-top: 6px; flex-shrink: 0; }
.event-dot.user { background: var(--color-accent); }
.event-dot.ai { background: var(--color-primary); }
.event-dot.system { background: var(--text-tertiary); }
.event-body { flex: 1; }
.event-action { font-size: var(--text-sm); font-weight: 500; }
.event-detail { font-size: var(--text-xs); color: var(--text-secondary); margin-top: 1px; }
.event-actor { font-size: var(--text-xs); color: var(--text-tertiary); }
</style>
