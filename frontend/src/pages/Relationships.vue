<template>
  <div class="relationships-page">
    <div class="page-header">
      <h1 class="page-title">关系管理</h1>
      <button class="btn-primary">+ 新建关系</button>
    </div>
    <div class="rel-content">
      <div class="rel-filters">
        <button v-for="f in filters" :key="f.id" class="filter-btn" :class="{ active: activeFilter === f.id }" @click="activeFilter = f.id">{{ f.label }}</button>
      </div>
      <div class="rel-list">
        <div v-for="rel in filteredRelations" :key="rel.id" class="rel-card">
          <div class="rel-header">
            <span class="rel-source">{{ rel.source }}</span>
            <span class="rel-arrow">→</span>
            <span class="rel-type" :class="rel.type">{{ rel.type }}</span>
            <span class="rel-arrow">→</span>
            <span class="rel-target">{{ rel.target }}</span>
          </div>
          <div class="rel-desc" v-if="rel.description">{{ rel.description }}</div>
          <div class="rel-meta">
            <span class="rel-time">{{ rel.time }}</span>
            <div class="rel-actions">
              <button class="action-btn">编辑</button>
              <button class="action-btn danger">删除</button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
const activeFilter = ref('all')
const filters = [
  { id: 'all', label: '全部' },
  { id: 'ally', label: '盟友' },
  { id: 'enemy', label: '敌对' },
  { id: 'located', label: '位于' },
  { id: 'belongs', label: '归属' },
]
const relations = [
  { id: '1', source: '林凡', target: '苏晚晴', type: 'ally', description: '共同冒险的伙伴', time: '1月20日' },
  { id: '2', source: '林凡', target: '王天德', type: 'enemy', description: '王家追杀林凡', time: '2月1日' },
  { id: '3', source: '林凡', target: '黑石城', type: 'located', description: '当前所在城市', time: '1月15日' },
  { id: '4', source: '王天德', target: '王家', type: 'belongs', description: '王家家主', time: '1月16日' },
  { id: '5', source: '苏晚晴', target: '地下遗迹', type: 'located', description: '与林凡同行', time: '3月12日' },
  { id: '6', source: '王家', target: '黑石城', type: 'belongs', description: '控制东区', time: '1月16日' },
  { id: '7', source: '黑市', target: '黑石城', type: 'located', description: '隐藏于地下', time: '1月25日' },
]
const filteredRelations = computed(() => {
  if (activeFilter.value === 'all') return relations
  return relations.filter(r => r.type === activeFilter.value)
})
</script>

<style scoped>
.relationships-page { height: 100%; overflow-y: auto; padding: var(--space-6) var(--space-8); }
.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--space-6); }
.page-title { font-size: var(--text-2xl); font-weight: 700; font-family: var(--font-serif); }
.btn-primary { padding: var(--space-2) var(--space-4); background: var(--color-primary); border: none; color: white; border-radius: var(--radius-sm); font-size: var(--text-sm); cursor: pointer; }
.btn-primary:hover { background: var(--color-primary-hover); }
.rel-content { }
.rel-filters { display: flex; gap: var(--space-2); margin-bottom: var(--space-4); }
.filter-btn { padding: var(--space-1) var(--space-3); border: 1px solid var(--border-default); background: transparent; color: var(--text-secondary); border-radius: var(--radius-sm); font-size: var(--text-xs); cursor: pointer; }
.filter-btn.active { background: var(--color-primary-subtle); border-color: var(--color-primary); color: var(--color-primary-text); }
.rel-list { display: flex; flex-direction: column; gap: var(--space-3); }
.rel-card { padding: var(--space-4); border: 1px solid var(--border-default); border-radius: var(--radius-md); background: var(--bg-panel); }
.rel-header { display: flex; align-items: center; gap: var(--space-2); margin-bottom: var(--space-2); }
.rel-source, .rel-target { font-weight: 500; }
.rel-arrow { color: var(--text-tertiary); }
.rel-type { font-size: var(--text-xs); padding: 2px 8px; border-radius: 10px; }
.rel-type.ally { background: var(--color-success-subtle); color: var(--color-success); }
.rel-type.enemy { background: var(--color-error-subtle); color: var(--color-error); }
.rel-type.located { background: var(--color-info-subtle); color: var(--color-info); }
.rel-type.belongs { background: var(--color-warning-subtle); color: var(--color-warning); }
.rel-desc { font-size: var(--text-sm); color: var(--text-secondary); margin-bottom: var(--space-2); }
.rel-meta { display: flex; justify-content: space-between; align-items: center; }
.rel-time { font-size: var(--text-xs); color: var(--text-tertiary); }
.rel-actions { display: flex; gap: var(--space-2); }
.action-btn { padding: var(--space-1) var(--space-2); border: 1px solid var(--border-default); background: transparent; color: var(--text-secondary); border-radius: var(--radius-sm); font-size: var(--text-xs); cursor: pointer; }
.action-btn:hover { background: var(--bg-hover); }
.action-btn.danger { color: var(--color-error); border-color: var(--color-error); }
.action-btn.danger:hover { background: var(--color-error-subtle); }
</style>
