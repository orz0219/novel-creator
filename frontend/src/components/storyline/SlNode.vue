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
        :class="{ collapsed: !expanded }"
        :title="expanded ? '折叠子线' : '展开子线'"
        :aria-expanded="expanded"
        @click.stop="emit('toggle')"
      >
        {{ expanded ? '−' : '+' }}
      </button>
      <span v-else class="sl-leaf" aria-hidden="true"></span>
    </div>

    <article
      class="sl-card"
      tabindex="0"
      @click="emit('edit', node)"
      @keydown.enter.self="emit('edit', node)"
    >
      <div class="sl-header">
        <span class="sl-importance" :class="importanceClass">
          {{ importanceLabel(node.importance) }}
        </span>
        <h3 class="sl-name">{{ node.name }}</h3>

        <div class="sl-actions" @click.stop>
          <button
            class="sl-action-btn add"
            title="在此节点下添加子线"
            @click="emit('add-child', node.id)"
          >
            <span class="plus" aria-hidden="true">＋</span>
            <span class="action-text">子线</span>
          </button>
          <button
            class="sl-action-btn del"
            title="删除剧情线"
            @click="emit('delete', node)"
          >
            删除
          </button>
        </div>
      </div>

      <div class="sl-meta">
        <span class="sl-tone" :class="node.tone">
          {{ node.tone === 'dark' ? '🌑 暗线' : '☀️ 明线' }}
        </span>
        <span v-if="node.visibility === 'hidden'" class="sl-visibility hidden">
          🔒 隐藏
        </span>
        <span class="sl-status" :class="statusClass">
          {{ statusLabel(node.status) }}
        </span>
      </div>

      <div v-if="sectionTitles.length" class="sl-section-chips">
        <span
          v-for="(title, index) in sectionTitles"
          :key="`${title}-${index}`"
          class="sl-section-chip"
        >
          {{ title }}
        </span>
      </div>

      <div
        v-if="descriptionSections.length"
        class="sl-desc"
        :class="{ 'is-clamped': !descExpanded && hasLongDescription }"
      >
        <div
          v-for="(section, index) in visibleSections"
          :key="index"
          class="sl-section"
        >
          <span v-if="section.title" class="sl-section-title">
            {{ section.title }}
          </span>
          <p class="sl-section-text">{{ section.content || '—' }}</p>
        </div>
      </div>

      <button
        v-if="hasLongDescription"
        class="sl-desc-toggle"
        @click.stop="descExpanded = !descExpanded"
      >
        {{ descExpanded ? '收起正文' : `展开正文 · ${descriptionSections.length} 段` }}
      </button>
    </article>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
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
const statusClass = computed(() => props.node.status.toLowerCase())

interface DescriptionSection {
  title: string
  content: string
}

const descExpanded = ref(false)

/** 把描述按【小标题】拆成结构块，避免一大段文字挤在一起 */
function parseDescription(raw?: string): DescriptionSection[] {
  const text = (raw ?? '').replace(/\r\n?/g, '\n').trim()
  if (!text) return []

  const marker = /【([^】\n]+)】/g
  const sections: DescriptionSection[] = []
  let cursor = 0
  let match: RegExpExecArray | null

  function appendToLast(chunk: string) {
    if (!chunk) return
    const last = sections[sections.length - 1]
    if (last) {
      last.content = [last.content, chunk].filter(Boolean).join('\n\n')
    } else {
      sections.push({ title: '', content: chunk })
    }
  }

  while ((match = marker.exec(text))) {
    appendToLast(text.slice(cursor, match.index).trim())
    sections.push({ title: match[1].trim(), content: '' })
    cursor = marker.lastIndex
  }
  appendToLast(text.slice(cursor).trim())

  return sections.filter((section) => section.title || section.content)
}

const descriptionSections = computed(() => parseDescription(props.node.description))
const sectionTitles = computed(() =>
  descriptionSections.value
    .map((section) => section.title)
    .filter((title) => Boolean(title)),
)
const plainTextLength = computed(() =>
  descriptionSections.value.reduce((total, section) => total + section.content.length, 0),
)
const hasLongDescription = computed(
  () => descriptionSections.value.length > 1 || plainTextLength.value > 140,
)
const visibleSections = computed(() => {
  if (descExpanded.value) return descriptionSections.value
  const firstTitled = descriptionSections.value.find((section) => section.title)
  return firstTitled ? [firstTitled] : descriptionSections.value.slice(0, 1)
})
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
/* ===== 2026-09 视觉重构：结构卡片 ===== */
.sl-row {
  align-items: flex-start;
}
.sl-indent {
  padding-top: 15px;
}
.sl-toggle {
  width: 22px;
  height: 22px;
  font-size: 16px;
  line-height: 1;
}
.sl-toggle.collapsed {
  background: var(--bg-panel-tertiary);
  border-color: var(--border-emphasis);
  color: var(--color-primary-text);
}
.sl-card {
  min-width: 0;
  padding: 12px 14px;
  transition: border-color var(--transition-fast), box-shadow var(--transition-fast), transform var(--transition-fast);
}
.sl-card:hover,
.sl-card:focus-visible {
  border-color: var(--color-primary);
  box-shadow: 0 4px 14px rgba(0, 0, 0, 0.22);
  transform: translateY(-1px);
  outline: none;
}
.is-main .sl-card {
  border: 1px solid var(--border-default);
  border-left: 3px solid var(--color-primary);
  background: linear-gradient(135deg, rgba(200, 75, 49, 0.1) 0%, rgba(200, 75, 49, 0.02) 55%, var(--bg-panel) 100%);
}
.is-main .sl-card:hover {
  box-shadow: 0 6px 20px rgba(200, 75, 49, 0.18);
}
.is-dark .sl-card {
  border-style: dashed;
  border-color: rgba(168, 85, 247, 0.45);
  background: linear-gradient(135deg, rgba(107, 33, 168, 0.1) 0%, rgba(107, 33, 168, 0.02) 55%, var(--bg-panel) 100%);
}
.is-dark .sl-card:hover {
  border-color: rgba(192, 132, 252, 0.75);
}

.sl-header {
  align-items: flex-start;
}
.sl-importance {
  flex-shrink: 0;
  margin-top: 1px;
  border-radius: 999px;
  line-height: 1.5;
}
.sl-importance.normal { background: rgba(139, 148, 158, 0.14); color: #aeb8c2; }
.sl-importance.minor { background: rgba(110, 118, 129, 0.12); color: #8b949e; }
.sl-name {
  flex: 1;
  min-width: 0;
  margin: 0;
  line-height: 1.45;
  overflow-wrap: anywhere;
}
.sl-actions {
  gap: 6px;
  flex-shrink: 0;
  margin-left: auto;
}
.sl-action-btn {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  width: auto;
  height: 26px;
  padding: 0 8px;
  border-radius: var(--radius-sm);
  color: var(--text-tertiary);
  font-size: var(--text-xs);
  font-family: inherit;
  white-space: nowrap;
}
.sl-action-btn .plus {
  font-size: 14px;
  line-height: 1;
}
.sl-action-btn.add:hover {
  color: var(--color-success);
  border-color: var(--color-success);
  background: var(--color-success-subtle);
}
.sl-action-btn.del:hover {
  color: var(--color-error);
  border-color: var(--color-error);
  background: var(--color-error-subtle);
}

.sl-meta {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 8px;
}
.sl-tone,
.sl-visibility,
.sl-status {
  border-radius: 999px;
  line-height: 1.5;
}
.sl-status { margin-left: auto; }
.sl-status.resolved { color: #60a5fa; }

.sl-section-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 10px;
  padding-top: 10px;
  border-top: 1px dashed var(--border-muted);
}
.sl-section-chip {
  max-width: 100%;
  padding: 2px 8px;
  border: 1px solid var(--border-muted);
  border-radius: 999px;
  background: var(--bg-panel-secondary);
  color: var(--text-secondary);
  font-size: 10px;
  line-height: 1.5;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.sl-section-chip::before {
  content: '#';
  margin-right: 3px;
  color: var(--color-primary);
  font-weight: 700;
}

.sl-desc {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: 10px;
}
.sl-section {
  min-width: 0;
}
.sl-section-title {
  display: inline-block;
  margin-bottom: 4px;
  padding: 1px 6px;
  border-radius: 4px;
  background: var(--color-primary-subtle);
  color: var(--color-primary-text);
  font-size: var(--text-xs);
  font-weight: 700;
  line-height: 1.6;
}
.sl-section-text {
  margin: 0;
  color: var(--text-secondary);
  font-size: var(--text-sm);
  line-height: 1.75;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}
.sl-desc.is-clamped .sl-section-text {
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.sl-desc-toggle {
  display: inline-flex;
  align-items: center;
  margin-top: 8px;
  padding: 2px 0;
  border: none;
  background: none;
  color: var(--color-primary-text);
  font-size: var(--text-xs);
  font-family: inherit;
  cursor: pointer;
}
.sl-desc-toggle:hover {
  color: var(--color-primary-hover);
  text-decoration: underline;
}
</style>
