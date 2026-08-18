<template>
  <div class="rules-page">
    <div class="page-header">
      <h1 class="page-title">世界规则</h1>
      <button class="btn-primary">+ 添加规则</button>
    </div>
    <div class="rules-content">
      <div class="rule-section">
        <h3 class="section-title">世界规则</h3>
        <div class="rule-list">
          <div v-for="rule in worldRules" :key="rule.id" class="rule-item">
            <span class="rule-severity" :class="rule.severity">{{ rule.severity }}</span>
            <span class="rule-text">{{ rule.text }}</span>
            <div class="rule-actions">
              <button class="action-btn">编辑</button>
              <button class="action-btn danger">删除</button>
            </div>
          </div>
        </div>
      </div>
      <div class="rule-section">
        <h3 class="section-title">世界设定</h3>
        <div class="setting-content">
          <div class="setting-item" v-for="setting in worldSettings" :key="setting.key">
            <span class="setting-key">{{ setting.key }}</span>
            <span class="setting-value">{{ setting.value }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
const worldRules = [
  { id: '1', text: '死人不能复活', severity: 'error' },
  { id: '2', text: '天玄大陆有三大帝国', severity: 'info' },
  { id: '3', text: '修炼等级：炼气→筑基→金丹→元婴→化神', severity: 'info' },
]
const worldSettings = [
  { key: '世界类型', value: '修仙世界' },
  { key: '时间体系', value: '天玄历' },
  { key: '货币体系', value: '灵石' },
]
</script>

<style scoped>
.rules-page { height: 100%; overflow-y: auto; padding: var(--space-6) var(--space-8); }
.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--space-6); }
.page-title { font-size: var(--text-2xl); font-weight: 700; font-family: var(--font-serif); }
.btn-primary { padding: var(--space-2) var(--space-4); background: var(--color-primary); border: none; color: white; border-radius: var(--radius-sm); font-size: var(--text-sm); cursor: pointer; }
.rules-content { max-width: 700px; }
.rule-section { margin-bottom: var(--space-8); }
.section-title { font-size: var(--text-md); font-weight: 600; margin-bottom: var(--space-4); padding-bottom: var(--space-2); border-bottom: 1px solid var(--border-muted); }
.rule-item { display: flex; align-items: center; gap: var(--space-3); padding: var(--space-3) 0; border-bottom: 1px solid var(--border-muted); }
.rule-severity { font-size: 10px; padding: 2px 6px; border-radius: 3px; text-transform: uppercase; }
.rule-severity.error { background: var(--color-error-subtle); color: var(--color-error); }
.rule-severity.info { background: var(--color-info-subtle); color: var(--color-info); }
.rule-text { flex: 1; font-size: var(--text-sm); }
.rule-actions { display: flex; gap: var(--space-2); }
.action-btn { padding: var(--space-1) var(--space-2); border: 1px solid var(--border-default); background: transparent; color: var(--text-secondary); border-radius: var(--radius-sm); font-size: var(--text-xs); cursor: pointer; }
.action-btn:hover { background: var(--bg-hover); }
.action-btn.danger { color: var(--color-error); }
.setting-item { display: flex; justify-content: space-between; padding: var(--space-3) 0; border-bottom: 1px solid var(--border-muted); }
.setting-key { font-size: var(--text-sm); color: var(--text-tertiary); }
.setting-value { font-size: var(--text-sm); color: var(--text-primary); }
</style>
