<!--
  SlNode.vue — 单个剧情线节点（树形视图的最小单位）

  Props:
    - node: Storyline（含 tone/visibility/importance/status）
    - depth: 当前深度（用于视觉缩进）
    - hasChildren: 是否有子副线（决定 [+/-] 按钮可见性）
    - expanded: 当前是否展开
  Emits:
    - toggle: 点击 +/- 时
    - edit(n): 点击卡片时
    - delete(n): 点击删除按钮时
    - addChild(parentId): 点击 "+子" 按钮时
-->
<template>
  <div
    class="sl-row"
    :class="[
      `depth-${depth}`,
      toneClass,
      visibilityClass,
      importanceClass,
    ]"
  >
    <div class="sl-indent">
      <button
        v-if="hasChildren"
        class="sl-toggle"
        :title="expanded ? '折叠' : '展开'"
        @click="emit('toggle')"
      >
        {{ expanded ? '−' : '+' }}
      </button>
      <span v-else class="sl-leaf">·</span>
    </div>

    <div class="sl-card" @click="emit('edit', node)">
      <div class="sl-header">
        <span class="sl-importance" :class="importanceClass">
          {{ importanceLabel(node.importance) }}
        </span>
        <span class="sl-name">{{ node.name }}</span>
        <span class="sl-tone" :class="node.tone">
          {{ node.tone === 'dark' ? '🌑 暗线' : '☀️ 明线' }}
        </span>
        <span v-if="node.visibility === 'hidden'" class="sl-visibility hidden">
          🔒 隐藏
        </span>
        <span class="sl-status" :class="node.status.toLowerCase()">
          {{ statusLabel(node.status) }}
        </span>
        <div class="sl-actions" @click.stop>
          <button
            class="sl-action-btn add"
            title="在此节点下挂载新副线"
            @click="emit('add-child', node.id)"
          >
            +
          </button>
          <button
            class="sl-action-btn del"
            title="删除"
            @click="emit('delete', node)"
          >
            ×
          </button>
        </div>
      </div>
      <div v-if="node.description" class="sl-desc">{{ node.description }}</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type {
  Storyline,
  StorylineStatus,
  StorylineImportance,
} from '@/types/narrative'

const props = defineProps<{
  node: Storyline
  depth: number
  hasChildren: boolean
  expanded: boolean
}>()

const emit = defineEmits<{
  toggle: []
  edit: [n: Storyline]
  delete: [n: Storyline]
  'add-child': [parentId: string]
}>()

const statusMap: Record<StorylineStatus, string> = {
  Planned: '计划中',
  Active: '进行中',
  Resolved: '已解决',
  Abandoned: '已放弃',
}
const importanceMap: Record<StorylineImportance, string> = {
  Main: '主线',
  Important: '重要',
  Normal: '普通',
  Minor: '次要',
}

function statusLabel(s: StorylineStatus) { return statusMap[s] ?? s }
function importanceLabel(i: StorylineImportance) { return importanceMap[i] ?? i }

const importanceClass = computed(() => props.node.importance.toLowerCase())
const toneClass = computed(() =>
  props.node.tone === 'dark' ? 'is-dark' : 'is-light',
)
const visibilityClass = computed(() =>
  props.node.visibility === 'hidden' ? 'is-hidden' : 'is-visible',
)
</script>

<style scoped>
.sl-row {
  display: flex;
  align-items: stretch;
  gap: 0;
  padding-left: 0;
}
.sl-indent {
  display: flex;
  align-items: flex-start;
  padding-top: 14px;
  width: 28px;
  flex-shrink: 0;
}
.sl-toggle {
  width: 22px;
  height: 22px;
  border-radius: 50%;
  border: 1px solid var(--border-default);
  background: var(--bg-base);
  color: var(--text-secondary);
  font-size: var(--text-md);
  font-family: var(--font-serif);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  flex-shrink: 0;
  transition: all var(--transition-fast);
}
.sl-toggle:hover { background: var(--bg-hover); border-color: var(--border-emphasis); }
.sl-leaf {
  width: 22px;
  height: 22px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-disabled);
  font-size: var(--text-md);
}

.sl-card {
  flex: 1;
  background: var(--bg-panel);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  padding: 10px 14px;
  cursor: pointer;
  margin: 4px 0;
  transition: all var(--transition-fast);
}
.sl-card:hover {
  border-color: var(--color-primary);
  box-shadow: 0 2px 8px rgba(200, 75, 49, 0.08);
}

/* 主线：朱砂红实色 */
.is-main .sl-card {
  border: 2px solid var(--color-primary);
  background: linear-gradient(135deg,
    rgba(200, 75, 49, 0.06) 0%,
    var(--bg-panel) 100%);
}
.is-main .sl-card:hover {
  box-shadow: 0 4px 16px rgba(200, 75, 49, 0.15);
}

/* 暗线：虚线 + 暗色背景 */
.is-dark .sl-card {
  border: 1px dashed #6b21a8;
  background: linear-gradient(135deg,
    rgba(107, 33, 168, 0.04) 0%,
    var(--bg-panel) 100%);
}
.is-dark .sl-card:hover {
  background: linear-gradient(135deg,
    rgba(107, 33, 168, 0.08) 0%,
    var(--bg-panel) 100%);
}

.is-hidden .sl-card { opacity: 0.85; }

.sl-header {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  flex-wrap: wrap;
}
.sl-importance {
  font-size: var(--text-xs);
  padding: 2px 8px;
  border-radius: 10px;
  font-family: var(--font-serif);
  font-weight: 600;
}
.sl-importance.main { background: var(--color-primary); color: white; }
.sl-importance.important { background: rgba(245, 158, 11, 0.15); color: #f59e0b; }
.sl-importance.normal { background: rgba(107, 114, 128, 0.15); color: #6b7280; }
.sl-importance.minor { background: rgba(0, 0, 0, 0.05); color: #9ca3af; }

.sl-name {
  font-weight: 600;
  font-size: var(--text-md);
  color: var(--text-primary);
  font-family: var(--font-serif);
}

.sl-tone {
  font-size: var(--text-xs);
  padding: 2px 8px;
  border-radius: 10px;
}
.sl-tone.light { background: rgba(245, 158, 11, 0.12); color: #d97706; }
.sl-tone.dark { background: rgba(107, 33, 168, 0.15); color: #c084fc; }

.sl-visibility {
  font-size: var(--text-xs);
  padding: 2px 8px;
  border-radius: 10px;
}
.sl-visibility.hidden { background: rgba(0, 0, 0, 0.05); color: #6b7280; }

.sl-status {
  font-size: var(--text-xs);
  padding: 2px 8px;
  border-radius: 10px;
  margin-left: auto;
}
.sl-status.planned { background: rgba(0, 0, 0, 0.05); color: var(--text-tertiary); }
.sl-status.active { background: rgba(63, 185, 80, 0.15); color: var(--color-success); }
.sl-status.resolved { background: rgba(59, 130, 246, 0.15); color: #3b82f6; }
.sl-status.abandoned { background: rgba(248, 81, 73, 0.12); color: var(--color-error); }

.sl-actions {
  display: flex;
  gap: 4px;
  margin-left: var(--space-2);
}
.sl-action-btn {
  width: 26px;
  height: 26px;
  border-radius: 50%;
  border: 1px solid var(--border-default);
  background: var(--bg-base);
  color: var(--text-tertiary);
  font-size: var(--text-sm);
  font-family: var(--font-serif);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  transition: all var(--transition-fast);
}
.sl-action-btn:hover { background: var(--bg-hover); color: var(--text-primary); border-color: var(--border-emphasis); }
.sl-action-btn.add:hover { color: var(--color-success); border-color: var(--color-success); }
.sl-action-btn.del:hover { color: var(--color-error); border-color: var(--color-error); }

.sl-desc {
  font-size: var(--text-sm);
  color: var(--text-secondary);
  margin-top: 6px;
  line-height: 1.6;
  white-space: pre-wrap;
}
</style>
