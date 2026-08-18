<template>
  <div class="version-diff">
    <div class="panel-header">
      <span class="panel-title">版本对比</span>
      <div class="version-selectors">
        <select v-model="fromVersion" class="version-select">
          <option v-for="v in versions" :key="v.id" :value="v.id">v{{ v.id }} - {{ v.label }}</option>
        </select>
        <span class="arrow">→</span>
        <select v-model="toVersion" class="version-select">
          <option v-for="v in versions" :key="v.id" :value="v.id">v{{ v.id }} - {{ v.label }}</option>
        </select>
      </div>
    </div>
    <div class="diff-content">
      <div v-for="change in changes" :key="change.field" class="diff-item">
        <div class="diff-field">{{ change.field }}</div>
        <div class="diff-old" v-if="change.old">
          <span class="diff-label">旧值:</span>
          <span class="diff-value removed">{{ change.old }}</span>
        </div>
        <div class="diff-new" v-if="change.new">
          <span class="diff-label">新值:</span>
          <span class="diff-value added">{{ change.new }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
const fromVersion = ref('3')
const toVersion = ref('5')
const versions = [
  { id: '5', label: '更新修炼等级' },
  { id: '4', label: 'AI Proposal #182' },
  { id: '3', label: '手动修改背景' },
  { id: '2', label: '初始创建' },
]
const changes = [
  { field: 'cultivation', old: '炼气二层', new: '炼气三层' },
  { field: 'location', old: '边境小镇', new: '黑石城' },
  { field: 'description', old: '一个普通的边境散修', new: '一个从边境走出的年轻散修，性格坚韧，内心善良。' },
]
</script>

<style scoped>
.version-diff { display: flex; flex-direction: column; }
.panel-header { padding: var(--space-3) var(--space-4); border-bottom: 1px solid var(--border-muted); }
.panel-title { font-size: var(--text-sm); font-weight: 600; display: block; margin-bottom: var(--space-2); }
.version-selectors { display: flex; align-items: center; gap: var(--space-2); }
.version-select { padding: var(--space-1) var(--space-2); background: var(--bg-base); border: 1px solid var(--border-default); border-radius: var(--radius-sm); color: var(--text-primary); font-size: var(--text-xs); }
.arrow { color: var(--text-tertiary); }
.diff-content { padding: var(--space-3) var(--space-4); }
.diff-item { margin-bottom: var(--space-3); padding: var(--space-3); border: 1px solid var(--border-muted); border-radius: var(--radius-sm); }
.diff-field { font-size: var(--text-sm); font-weight: 600; margin-bottom: var(--space-2); font-family: var(--font-mono); }
.diff-old, .diff-new { display: flex; gap: var(--space-2); font-size: var(--text-sm); margin-bottom: var(--space-1); }
.diff-label { color: var(--text-tertiary); min-width: 40px; }
.diff-value { font-family: var(--font-mono); padding: 2px 6px; border-radius: 3px; }
.diff-value.removed { background: var(--color-error-subtle); color: var(--color-error); text-decoration: line-through; }
.diff-value.added { background: var(--color-success-subtle); color: var(--color-success); }
</style>
