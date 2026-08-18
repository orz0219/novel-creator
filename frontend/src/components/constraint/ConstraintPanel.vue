<template>
  <div class="constraint-panel">
    <div class="panel-header">
      <span class="panel-title">世界约束 (Constraints)</span>
      <button class="add-btn" @click="showAdd = true">+ 添加</button>
    </div>
    <div class="constraint-levels">
      <div v-for="level in levels" :key="level.id" class="constraint-level">
        <div class="level-header" @click="toggleLevel(level.id)">
          <span class="expand-icon">{{ expanded[level.id] ? '▼' : '▶' }}</span>
          <span class="level-name">{{ level.name }}</span>
          <span class="level-count">{{ level.constraints.length }}</span>
        </div>
        <div v-if="expanded[level.id]" class="level-body">
          <div v-for="c in level.constraints" :key="c.id" class="constraint-item">
            <span class="c-severity" :class="c.severity">{{ c.severity }}</span>
            <span class="c-text">{{ c.text }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
const showAdd = ref(false)
const expanded = ref<Record<string, boolean>>({ system: true, world: true, story: true, character: false, scene: false })

const levels = [
  {
    id: 'system', name: '系统级', constraints: [
      { id: 's1', text: '不能修改已经 Commit 的历史', severity: 'error' },
    ]
  },
  {
    id: 'world', name: '世界级', constraints: [
      { id: 'w1', text: '死人不能复活', severity: 'error' },
      { id: 'w2', text: '天玄大陆有三大帝国', severity: 'info' },
    ]
  },
  {
    id: 'story', name: '故事级', constraints: [
      { id: 'st1', text: '第三卷结束前不能揭露幕后 Boss', severity: 'warning' },
      { id: 'st2', text: '主角不能在黑石城死亡', severity: 'error' },
    ]
  },
  {
    id: 'character', name: '角色级', constraints: [
      { id: 'c1', text: '林凡不会主动杀无辜者', severity: 'warning' },
      { id: 'c2', text: '王天德表面儒雅实则狠辣', severity: 'info' },
    ]
  },
  {
    id: 'scene', name: '场景级', constraints: [
      { id: 'sc1', text: '当前场景必须发生在黑石城', severity: 'info' },
    ]
  },
]

function toggleLevel(id: string) { expanded.value[id] = !expanded.value[id] }
</script>

<style scoped>
.constraint-panel { display: flex; flex-direction: column; }
.panel-header { display: flex; justify-content: space-between; align-items: center; padding: var(--space-3) var(--space-4); border-bottom: 1px solid var(--border-muted); }
.panel-title { font-size: var(--text-sm); font-weight: 600; }
.add-btn { padding: var(--space-1) var(--space-2); border: 1px solid var(--border-default); background: transparent; color: var(--text-secondary); border-radius: var(--radius-sm); font-size: var(--text-xs); cursor: pointer; }
.add-btn:hover { background: var(--bg-hover); }
.constraint-levels { padding: var(--space-2); }
.constraint-level { margin-bottom: var(--space-1); }
.level-header { display: flex; align-items: center; gap: var(--space-2); padding: var(--space-2); cursor: pointer; border-radius: var(--radius-sm); font-size: var(--text-sm); }
.level-header:hover { background: var(--bg-hover); }
.expand-icon { font-size: var(--text-xs); color: var(--text-tertiary); width: 12px; }
.level-name { font-weight: 500; }
.level-count { margin-left: auto; font-size: var(--text-xs); color: var(--text-tertiary); }
.level-body { padding-left: var(--space-6); }
.constraint-item { display: flex; gap: var(--space-2); padding: var(--space-1) 0; font-size: var(--text-sm); }
.c-severity { font-size: 10px; padding: 2px 6px; border-radius: 3px; flex-shrink: 0; text-transform: uppercase; }
.c-severity.error { background: var(--color-error-subtle); color: var(--color-error); }
.c-severity.warning { background: var(--color-warning-subtle); color: var(--color-warning); }
.c-severity.info { background: var(--color-info-subtle); color: var(--color-info); }
.c-text { color: var(--text-secondary); }
</style>
