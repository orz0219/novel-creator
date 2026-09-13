<!--
  档案面板（人物 / 势力 / 地点共用）

  设计原则：**档案是要给人读的，编辑是次要动作。**

  旧实现打开就是十来个大小一致的输入框，问题有三：
  1. 空框一片，分不清哪些填了、哪些没填；
  2. 控件与内容不匹配（「领袖」填两个字也给一个大文本框，且固定 rows=2）；
  3. 字段等权重平铺，读的人抓不到结构。

  因此这里做成两态：
  - 阅读态（默认）：按分组呈现「标签 + 内容」，未填的显示浅色占位；
  - 编辑态（点「编辑」进入）：整块变成输入框，同时出现「保存 / 取消」。
-->
<template>
  <section class="profile-panel">
    <header class="panel-head">
      <div class="head-main">
        <h2 class="panel-title">{{ title }}</h2>
        <span v-if="subtitle" class="panel-subtitle">{{ subtitle }}</span>
      </div>
      <div class="head-actions">
        <template v-if="editing">
          <button class="btn-ghost" :disabled="saving" @click="cancelEdit">取消</button>
          <button class="btn-primary" :disabled="saving" @click="submit">
            {{ saving ? '保存中…' : '保存' }}
          </button>
        </template>
        <template v-else>
          <button v-if="extraLabel" class="btn-ghost" @click="$emit('extra')">{{ extraLabel }}</button>
          <button class="btn-ghost" @click="startEdit">编辑</button>
          <button v-if="showClose" class="btn-ghost" @click="$emit('close')">收起</button>
        </template>
      </div>
    </header>

    <div v-for="group in groups" :key="group.title" class="panel-group">
      <h3 class="group-title">{{ group.title }}</h3>

      <div class="field-list">
        <div v-for="f in group.fields" :key="f.key" class="field-row" :class="{ 'is-multiline': f.multiline }">
          <label class="field-label" :for="`pf-${f.key}`">{{ f.label }}</label>

          <!-- 阅读态：把值当内容读，未填单独给一个弱化占位 -->
          <div v-if="!editing" class="field-value">
            <!-- 对象数组（冲突 / 秘密 / 阶段弧线）：逐条列出，带角标；
                 若字段声明了 subFields，再在条目下方展开细节 -->
            <ul v-if="f.shape === 'object-list' && objectItems(f.key).length" class="value-list">
              <li v-for="(item, i) in objectItems(f.key)" :key="i" class="value-item">
                <div class="value-item-main">
                  <span v-if="badgeOf(f, item)" class="value-badge">{{ badgeOf(f, item) }}</span>
                  <span class="value-text">{{ textOf(item, f.primaryKey) }}</span>
                </div>
                <dl v-if="f.subFields?.length" class="value-dl value-dl-nested">
                  <template v-for="sub in f.subFields" :key="sub.key">
                    <template v-if="hasValue(item[sub.key])">
                      <dt>{{ sub.label }}</dt>
                      <dd>{{ flat(item[sub.key]) }}</dd>
                    </template>
                  </template>
                </dl>
              </li>
            </ul>
            <!-- 对象（驱动力 / 能力边界 / 弧光）：逐子字段列出 -->
            <dl v-else-if="f.shape === 'object' && hasSubValues(f)" class="value-dl">
              <template v-for="sub in f.subFields || []" :key="sub.key">
                <dt v-if="hasValue(subValue(f.key, sub.key))">{{ sub.label }}</dt>
                <dd v-if="hasValue(subValue(f.key, sub.key))">{{ flat(subValue(f.key, sub.key)) }}</dd>
              </template>
            </dl>
            <!-- 字符串数组 -->
            <ul v-else-if="f.shape === 'string-list' && stringItems(f.key).length" class="value-list">
              <li v-for="(s, i) in stringItems(f.key)" :key="i" class="value-item">
                <span class="value-text">{{ s }}</span>
              </li>
            </ul>
            <!-- 默认：单行 / 多行文本 -->
            <template v-else>
              <span v-if="display(f.key)" class="value-text">{{ display(f.key) }}</span>
              <span v-else class="value-empty">未填写</span>
            </template>
          </div>

          <!-- 编辑态：控件按内容长度选，长字段自动长高 -->
          <div v-else class="field-control">
            <p v-if="f.readOnly" class="field-readonly">
              这一项由 AI 维护（结构较复杂，手工改容易出错）——在对话里请它补充即可
            </p>
            <select
              v-else-if="f.options"
              :id="`pf-${f.key}`"
              v-model="draft[f.key] as string"
              class="ctl ctl-select"
            >
              <option v-for="o in f.options" :key="o.value" :value="o.value">{{ o.label }}</option>
            </select>
            <textarea
              v-else-if="f.multiline"
              :id="`pf-${f.key}`"
              v-model="draft[f.key] as string"
              class="ctl ctl-textarea"
              rows="1"
              :placeholder="f.placeholder || `填写${f.label}`"
              @input="autoGrow"
            ></textarea>
            <input
              v-else
              :id="`pf-${f.key}`"
              v-model="draft[f.key] as string"
              class="ctl ctl-input"
              type="text"
              :placeholder="f.placeholder || `填写${f.label}`"
            />
            <span v-if="f.hint && !f.readOnly" class="field-hint">{{ f.hint }}</span>
          </div>
        </div>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { ref, nextTick } from 'vue'
import { displayProfileValue } from '@/utils/profileDisplay'

export interface ProfileSubField {
  key: string
  label: string
}

/** 值的展示形态。结构化字段（冲突 / 驱动力 / 秘密）不能用一段文本渲染。 */
export type ProfileFieldShape = 'text' | 'object' | 'object-list' | 'string-list'

export interface ProfileField {
  key: string
  label: string
  /** 长文本字段：用多行框并随内容长高 */
  multiline?: boolean
  /** 枚举字段：用下拉（后端只认规范值） */
  options?: { value: string; label: string }[]
  hint?: string
  placeholder?: string
  /** 值的展示形态，默认 text */
  shape?: ProfileFieldShape
  /** shape=object 时的子字段（含中文标签） */
  subFields?: ProfileSubField[]
  /** shape=object-list 时，取哪个键作为主文本 */
  primaryKey?: string
  /** shape=object-list 时，取哪个键作为角标 */
  badgeKey?: string
  /** 角标前缀（如「重要度 」），用于数值型角标 */
  badgePrefix?: string
  /** 角标值的显示映射（如 Internal → 内在） */
  badgeLabels?: Record<string, string>
  /**
   * 只读：编辑态下不提供输入框。
   * 用于结构复杂的字段（对象数组等）——手工在输入框里改 JSON 极容易把数据弄坏，
   * 这类字段交给 AI 用工具维护。
   */
  readOnly?: boolean
}

export interface ProfileGroup {
  title: string
  fields: ProfileField[]
}

const props = withDefaults(
  defineProps<{
    title: string
    subtitle?: string
    groups: ProfileGroup[]
    /** 字段值集合；键与 ProfileField.key 对应 */
    modelValue: Record<string, unknown>
    saving?: boolean
    /** 阅读态下额外显示的按钮文案（如「编辑基础信息」） */
    extraLabel?: string
    /** 是否显示「收起」：同页面有多个面板时只让第一个显示，避免按钮重复 */
    showClose?: boolean
  }>(),
  { showClose: true },
)

const emit = defineEmits<{
  save: [value: Record<string, unknown>]
  close: []
  extra: []
}>()

const editing = ref(false)
const draft = ref<Record<string, unknown>>({})

/**
 * 字段值的展示文本。
 *
 * 枚举字段的存储值是英文（后端契约要求 `Adult` / `Male` / `Protagonist`），
 * 但界面上必须显示中文标签——映射逻辑与用例见 `utils/profileDisplay.ts`。
 */
function display(key: string): string {
  const fields = props.groups.flatMap((g) => g.fields)
  return displayProfileValue(key, props.modelValue[key], fields)
}

/** 值是否有内容（空串 / 空数组 / 空对象都算没有）。 */
function hasValue(v: unknown): boolean {
  if (v === null || v === undefined) return false
  if (Array.isArray(v)) return v.length > 0
  if (typeof v === 'object') return Object.keys(v as object).length > 0
  return String(v).trim() !== ''
}

/** 数组用顿号连接，其余转字符串。 */
function flat(v: unknown): string {
  if (Array.isArray(v)) return v.filter(Boolean).map(String).join('、')
  if (v === null || v === undefined) return ''
  return String(v)
}

/** 取对象数组（缺失或类型不符时为空数组，由模板渲染「未填写」）。 */
function objectItems(key: string): Record<string, unknown>[] {
  const v = props.modelValue[key]
  if (!Array.isArray(v)) return []
  return v.filter((x): x is Record<string, unknown> => typeof x === 'object' && x !== null)
}

function stringItems(key: string): string[] {
  const v = props.modelValue[key]
  if (!Array.isArray(v)) return []
  return v.map((x) => String(x)).filter((s) => s.trim() !== '')
}

/** 对象形态下的子字段值。 */
function subValue(key: string, subKey: string): unknown {
  const v = props.modelValue[key]
  if (typeof v !== 'object' || v === null || Array.isArray(v)) return undefined
  return (v as Record<string, unknown>)[subKey]
}

function hasSubValues(f: ProfileField): boolean {
  return (f.subFields || []).some((sub) => hasValue(subValue(f.key, sub.key)))
}

/** 对象数组某条的主文本。 */
function textOf(item: Record<string, unknown>, primaryKey?: string): string {
  if (!primaryKey) return ''
  return flat(item[primaryKey])
}

/** 对象数组某条的角标（经 badgeLabels 映射成中文；映射不到就原样显示）。 */
function badgeOf(f: ProfileField, item: Record<string, unknown>): string {
  if (!f.badgeKey) return ''
  const raw = item[f.badgeKey]
  if (raw === null || raw === undefined || raw === '') return ''
  const text = String(raw)
  return (f.badgePrefix || '') + (f.badgeLabels?.[text] ?? text)
}

function startEdit() {
  draft.value = { ...props.modelValue }
  editing.value = true
  nextTick(() => {
    document.querySelectorAll<HTMLTextAreaElement>('.profile-panel textarea.ctl-textarea').forEach(autoGrowEl)
  })
}

function cancelEdit() {
  editing.value = false
  draft.value = {}
}

function submit() {
  emit('save', { ...draft.value })
}

/** 让多行框随内容长高（固定 rows 会留大片空白，长文又要手动拖） */
function autoGrow(e: Event) {
  autoGrowEl(e.target as HTMLTextAreaElement)
}

function autoGrowEl(el: HTMLTextAreaElement) {
  el.style.height = 'auto'
  el.style.height = `${el.scrollHeight}px`
}

/** 父组件保存成功后调用，退出编辑态 */
function finishEdit() {
  editing.value = false
  draft.value = {}
}

defineExpose({ finishEdit })
</script>

<style scoped>
.profile-panel {
  margin-top: var(--space-6);
  padding: var(--space-5) var(--space-6);
  background: var(--bg-panel);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-lg);
}

/* ---------- 头部 ---------- */
.panel-head {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding-bottom: var(--space-4);
  border-bottom: 1px solid var(--border-muted);
  margin-bottom: var(--space-5);
}
.head-main {
  display: flex;
  align-items: baseline;
  gap: var(--space-3);
  min-width: 0;
}
.panel-title {
  margin: 0;
  font-family: var(--font-serif);
  font-size: var(--text-lg);
  font-weight: 700;
  color: var(--text-primary);
}
.panel-subtitle {
  font-size: var(--text-sm);
  color: var(--color-primary-text);
}
.head-actions {
  margin-left: auto;
  display: flex;
  gap: var(--space-2);
  flex-shrink: 0;
}
.btn-ghost {
  padding: var(--space-1) var(--space-3);
  background: transparent;
  border: 1px solid var(--border-default);
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  font-size: var(--text-xs);
  cursor: pointer;
  transition: all var(--transition-fast);
}
.btn-ghost:hover:not(:disabled) {
  border-color: var(--border-emphasis);
  background: var(--bg-hover);
  color: var(--text-primary);
}
.btn-ghost:disabled {
  opacity: 0.5;
  cursor: default;
}
.btn-primary {
  padding: var(--space-1) var(--space-4);
  background: var(--color-primary);
  border: 1px solid var(--color-primary);
  border-radius: var(--radius-sm);
  color: #fff;
  font-size: var(--text-xs);
  cursor: pointer;
  transition: all var(--transition-fast);
}
.btn-primary:hover:not(:disabled) {
  background: var(--color-primary-hover);
}
.btn-primary:disabled {
  opacity: 0.6;
  cursor: default;
}

/* ---------- 分组 ---------- */
.panel-group + .panel-group {
  margin-top: var(--space-6);
}
.group-title {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin: 0 0 var(--space-3);
  font-size: var(--text-sm);
  font-weight: 600;
  color: var(--text-primary);
}
/* 分组标题前的主色短竖线：让结构一眼可辨，不用靠加粗撑层级 */
.group-title::before {
  content: '';
  width: 2px;
  height: 12px;
  border-radius: 1px;
  background: var(--color-primary);
}

/* ---------- 字段 ---------- */
.field-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
.field-row {
  display: grid;
  grid-template-columns: 88px 1fr;
  gap: var(--space-3);
  align-items: start;
  padding: var(--space-2) var(--space-3);
  border-radius: var(--radius-sm);
  transition: background var(--transition-fast);
}
/* 长文本字段改为上下结构：横向排列会让长内容挤在窄列里 */
.field-row.is-multiline {
  grid-template-columns: 88px 1fr;
}
.field-row:hover {
  background: var(--bg-hover);
}
.field-label {
  padding-top: 3px;
  font-size: var(--text-xs);
  color: var(--text-tertiary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.field-value {
  min-width: 0;
}
.value-text {
  font-size: var(--text-sm);
  line-height: var(--leading-relaxed);
  color: var(--text-primary);
  white-space: pre-wrap;
  word-break: break-word;
}
/* 未填写：弱化但可见，让人知道"这栏存在、只是还没写"，而不是以为坏了 */
.value-empty {
  font-size: var(--text-sm);
  color: var(--text-disabled);
  font-style: italic;
}

/* ---------- 结构化字段（冲突 / 秘密 / 驱动力）---------- */
.value-list {
  margin: 0;
  padding: 0;
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
.value-item {
  display: flex;
  flex-direction: column;
  align-items: stretch;
  gap: var(--space-1);
}
/* 条目标题行：角标 + 主文本 */
.value-item-main {
  display: flex;
  align-items: baseline;
  gap: var(--space-2);
}
/* 冲突类型 / 重要度这类角标：小、弱，不抢正文 */
.value-badge {
  flex-shrink: 0;
  padding: 1px 6px;
  border-radius: 3px;
  background: var(--bg-panel-tertiary);
  color: var(--text-tertiary);
  font-size: var(--text-xs);
  white-space: nowrap;
}
.value-dl {
  margin: 0;
  display: grid;
  grid-template-columns: 76px 1fr;
  gap: var(--space-1) var(--space-3);
}
/* 对象数组条目内的细节：缩进一点，和标题行区分开 */
.value-dl-nested {
  margin-left: 2px;
  grid-template-columns: 64px 1fr;
}
.value-dl dt {
  font-size: var(--text-xs);
  color: var(--text-tertiary);
  padding-top: 2px;
  white-space: nowrap;
}
.value-dl dd {
  margin: 0;
  font-size: var(--text-sm);
  line-height: var(--leading-relaxed);
  color: var(--text-primary);
  white-space: pre-wrap;
  word-break: break-word;
}
/* 只读字段在编辑态的说明 */
.field-readonly {
  margin: 0;
  padding: var(--space-2) var(--space-3);
  border-left: 2px solid var(--border-default);
  border-radius: var(--radius-sm);
  background: var(--bg-hover);
  font-size: var(--text-xs);
  line-height: var(--leading-normal);
  color: var(--text-tertiary);
}
.field-control {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  min-width: 0;
}
.field-hint {
  font-size: var(--text-xs);
  color: var(--text-tertiary);
}

/* ---------- 编辑控件 ---------- */
.ctl {
  width: 100%;
  padding: var(--space-2) var(--space-3);
  background: var(--bg-base);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: var(--text-sm);
  font-family: inherit;
  outline: none;
  transition: border-color var(--transition-fast);
}
.ctl:focus {
  border-color: var(--color-primary);
}
.ctl-select {
  cursor: pointer;
}
.ctl-textarea {
  resize: none;
  overflow: hidden;
  line-height: var(--leading-relaxed);
  min-height: 34px;
}
</style>
