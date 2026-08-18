<template>
  <InspectorPanel entity-type="Character" :entity-name="character.name" @close="$emit('close')">
    <template #default="{ activeTab }">
      <div v-if="activeTab === 'overview'" class="inspector-section">
        <div class="info-grid">
          <div class="info-row"><span class="label">身份</span><span>{{ character.attributes?.identity || '-' }}</span></div>
          <div class="info-row"><span class="label">年龄</span><span>{{ character.attributes?.age || '-' }}</span></div>
          <div class="info-row"><span class="label">性别</span><span>{{ character.attributes?.gender || '-' }}</span></div>
          <div class="info-row"><span class="label">修为</span><span>{{ character.attributes?.cultivation || '-' }}</span></div>
        </div>
        <div class="desc-section" v-if="character.description">
          <div class="section-label">描述</div>
          <p class="desc-text">{{ character.description }}</p>
        </div>
      </div>
      <div v-if="activeTab === 'knowledge'" class="inspector-section">
        <div class="section-label">角色知识状态</div>
        <div v-for="k in knowledge" :key="k.fact" class="knowledge-item" :class="k.level">
          <span class="k-badge">{{ k.level }}</span>
          <span class="k-fact">{{ k.fact }}</span>
        </div>
      </div>
      <div v-if="activeTab === 'relations'" class="inspector-section">
        <div class="section-label">关系</div>
        <div v-for="rel in relations" :key="rel.id" class="relation-item">
          <span>{{ rel.target }}</span>
          <span class="rel-type">{{ rel.type }}</span>
        </div>
      </div>
      <div v-if="activeTab === 'history'" class="inspector-section">
        <div class="section-label">版本历史</div>
        <div class="version-list">
          <div v-for="v in versions" :key="v.version" class="version-item">
            <span class="v-num">v{{ v.version }}</span>
            <span class="v-desc">{{ v.description }}</span>
            <span class="v-time">{{ v.time }}</span>
          </div>
        </div>
      </div>
    </template>
  </InspectorPanel>
</template>

<script setup lang="ts">
import InspectorPanel from './InspectorPanel.vue'
import type { Entity } from '@/types'
const props = defineProps<{ character: Entity }>()
defineEmits(['close'])

const knowledge = [
  { fact: '王家正在追杀自己', level: 'Known' },
  { fact: '王家真正目的', level: 'Unknown' },
  { fact: '苏晚晴与王家有关', level: 'Suspected' },
  { fact: '城主已经死亡', level: 'FalseBelief' },
]

const relations = [
  { id: '1', target: '苏晚晴', type: '同伴' },
  { id: '2', target: '王天德', type: '敌对' },
  { id: '3', target: '黑石城', type: '位于' },
]

const versions = [
  { version: 5, description: '更新修炼等级', time: '3月12日' },
  { version: 4, description: 'AI Proposal #182', time: '3月10日' },
  { version: 3, description: '手动修改背景', time: '2月28日' },
  { version: 2, description: '初始创建', time: '1月15日' },
]
</script>

<style scoped>
.inspector-section { margin-bottom: var(--space-4); }
.section-label { font-size: var(--text-xs); font-weight: 600; text-transform: uppercase; color: var(--text-tertiary); margin-bottom: var(--space-2); }
.info-grid { display: flex; flex-direction: column; gap: var(--space-2); }
.info-row { display: flex; justify-content: space-between; font-size: var(--text-sm); }
.label { color: var(--text-tertiary); }
.desc-text { font-size: var(--text-sm); color: var(--text-secondary); line-height: var(--leading-relaxed); }
.knowledge-item { display: flex; gap: var(--space-2); padding: var(--space-2) 0; border-bottom: 1px solid var(--border-muted); font-size: var(--text-sm); }
.k-badge { font-size: 10px; padding: 2px 6px; border-radius: 3px; flex-shrink: 0; }
.knowledge-item.Known .k-badge { background: var(--color-success-subtle); color: var(--color-success); }
.knowledge-item.Unknown .k-badge { background: var(--bg-panel-secondary); color: var(--text-tertiary); }
.knowledge-item.Suspected .k-badge { background: var(--color-warning-subtle); color: var(--color-warning); }
.knowledge-item.FalseBelief .k-badge { background: var(--color-error-subtle); color: var(--color-error); }
.k-fact { color: var(--text-secondary); }
.relation-item { display: flex; justify-content: space-between; padding: var(--space-2) 0; font-size: var(--text-sm); }
.rel-type { color: var(--color-primary-text); font-size: var(--text-xs); }
.version-item { display: flex; gap: var(--space-3); padding: var(--space-2) 0; border-bottom: 1px solid var(--border-muted); font-size: var(--text-sm); }
.v-num { font-family: var(--font-mono); color: var(--text-tertiary); min-width: 30px; }
.v-desc { flex: 1; color: var(--text-secondary); }
.v-time { color: var(--text-tertiary); font-size: var(--text-xs); }
</style>
