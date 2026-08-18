<template>
  <div class="knowledge-panel">
    <div class="panel-header">
      <span class="panel-title">知识状态</span>
      <span class="panel-subtitle">{{ characterName }}</span>
    </div>
    <div class="knowledge-sections">
      <div class="k-section">
        <div class="k-section-header">
          <span class="k-dot known"></span>
          <span>已知 (Known)</span>
          <span class="k-count">{{ knownItems.length }}</span>
        </div>
        <div v-for="item in knownItems" :key="item.fact" class="k-item">
          <span class="k-content">{{ item.fact }}</span>
          <span class="k-source">来源: {{ item.source }}</span>
        </div>
      </div>
      <div class="k-section">
        <div class="k-section-header">
          <span class="k-dot suspected"></span>
          <span>怀疑 (Suspected)</span>
          <span class="k-count">{{ suspectedItems.length }}</span>
        </div>
        <div v-for="item in suspectedItems" :key="item.fact" class="k-item">
          <span class="k-content">{{ item.fact }}</span>
          <span class="k-confidence">置信度: {{ item.confidence }}</span>
        </div>
      </div>
      <div class="k-section">
        <div class="k-section-header">
          <span class="k-dot unknown"></span>
          <span>未知 (Unknown)</span>
          <span class="k-count">{{ unknownItems.length }}</span>
        </div>
        <div v-for="item in unknownItems" :key="item" class="k-item">
          <span class="k-content">{{ item }}</span>
        </div>
      </div>
      <div class="k-section">
        <div class="k-section-header">
          <span class="k-dot false-belief"></span>
          <span>错误认知 (False Belief)</span>
          <span class="k-count">{{ falseBeliefs.length }}</span>
        </div>
        <div v-for="item in falseBeliefs" :key="item.belief" class="k-item">
          <span class="k-content">{{ item.belief }}</span>
          <span class="k-truth">真相: {{ item.truth }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  characterName: string
}>()

const knownItems = [
  { fact: '王家正在追杀自己', source: '亲身经历' },
  { fact: '古井旁有神秘令牌', source: '亲眼所见' },
  { fact: '苏晚晴是游方修士', source: '苏晚晴自述' },
]

const suspectedItems = [
  { fact: '苏晚晴与王家有关', confidence: '中等' },
  { fact: '地下遗迹有宝物', confidence: '低' },
]

const unknownItems = [
  '王家追杀的真正目的',
  '苏晚晴的真实身份',
  '古井背后的秘密',
]

const falseBeliefs = [
  { belief: '城主已经死亡', truth: '城主仍然存活，被王家软禁' },
]
</script>

<style scoped>
.knowledge-panel { display: flex; flex-direction: column; }
.panel-header { padding: var(--space-3) var(--space-4); border-bottom: 1px solid var(--border-muted); }
.panel-title { font-size: var(--text-sm); font-weight: 600; }
.panel-subtitle { font-size: var(--text-xs); color: var(--text-tertiary); margin-left: var(--space-2); }
.knowledge-sections { padding: var(--space-3); }
.k-section { margin-bottom: var(--space-4); }
.k-section-header { display: flex; align-items: center; gap: var(--space-2); margin-bottom: var(--space-2); font-size: var(--text-sm); font-weight: 500; }
.k-dot { width: 8px; height: 8px; border-radius: 50%; }
.k-dot.known { background: var(--color-success); }
.k-dot.suspected { background: var(--color-warning); }
.k-dot.unknown { background: var(--text-tertiary); }
.k-dot.false-belief { background: var(--color-error); }
.k-count { margin-left: auto; font-size: var(--text-xs); color: var(--text-tertiary); }
.k-item { padding: var(--space-2) 0; border-bottom: 1px solid var(--border-muted); }
.k-content { font-size: var(--text-sm); color: var(--text-secondary); display: block; }
.k-source, .k-confidence, .k-truth { font-size: var(--text-xs); color: var(--text-tertiary); }
.k-truth { color: var(--color-error); }
</style>
