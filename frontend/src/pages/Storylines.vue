<!--
  Storylines.vue — 剧情线页面（树形视图）

  树形结构：
    - Main 主线 = 树干（每项目 1 条）
    - 副线 = 树枝（挂到主线或其他副线）
    - 副线可以是 明线（light）/ 暗线（dark+hidden）

  视觉：
    - 缩进：每深一级左缩进 28px
    - 连接线：每条 child 前面用 CSS border-left 画竖线
    - 主线根：朱砂红实色边
    - 明线副线：常规样式
    - 暗线副线：虚线边框 + 暗色背景 + 🔒 角标
    - 展开/折叠：父节点有 [+] / [-] 切换；默认展开全部
-->
<template>
  <div class="storylines-page">
    <div class="page-header">
      <h1 class="page-title">剧情线</h1>
      <div class="header-actions">
        <span class="header-hint">
          共 {{ storyStore.storylines.length }} 条 · 副线 {{ branchCount }} 条 · 暗线 {{ darkCount }} 条
        </span>
        <button class="btn-ghost" @click="expandAll">全部展开</button>
        <button class="btn-ghost" @click="collapseAll">全部收起</button>
        <button class="btn-primary" @click="openCreate">+ 新建剧情线</button>
      </div>
    </div>
    <p class="page-tip">
      点击卡片编辑；左侧 <b>−/+</b> 折叠或展开；卡片右上角 <b>＋ 子线</b> 添加下级；虚线卡片为暗线。
    </p>
    <div v-if="storyStore.error" class="error-banner">{{ storyStore.error }}</div>

    <!-- 空状态 -->
    <div v-if="storyStore.storylines.length === 0" class="empty-state">
      <div class="empty-icon">🌳</div>
      <p class="empty-title">还没有剧情线</p>
      <p class="empty-sub">先创建一条 Main 主线（整本书的核心任务），然后再挂副线</p>
      <button class="btn-primary" @click="openCreate">+ 新建第一条剧情线</button>
    </div>

    <!-- 树形 -->
    <div v-else class="storyline-tree">
      <div v-if="mainLine" class="tree-root">
        <SlNode
          :node="mainLine"
          :depth="0"
          :has-children="childrenMap[mainLine.id]?.length > 0"
          :expanded="expandedIds[mainLine.id] !== false"
          @toggle="toggleExpand(mainLine.id)"
          @edit="openEdit"
          @delete="handleDelete"
          @add-child="openCreateChild"
        />
        <!-- 根节点折叠时，整棵副线树枝一起隐藏 -->
        <div v-if="expandedIds[mainLine.id] !== false" class="tree-branches">
          <div v-if="childrenMap[mainLine.id]?.length === 0" class="hint-no-branches">
            还没有副线 —
            <a class="link" @click="openCreateChild(mainLine.id)">新建一条</a>
          </div>
          <!-- 递归副线 -->
          <SlBranch
            v-for="(child, i) in childrenMap[mainLine.id] || []"
            :key="child.id"
            :node="child"
            :depth="1"
            :is-last="i === (childrenMap[mainLine.id]?.length ?? 0) - 1"
            :children-map="childrenMap"
            :expanded-ids="expandedIds"
            @toggle="toggleExpand"
            @edit="openEdit"
            @delete="handleDelete"
            @add-child="openCreateChild"
          />
        </div>
      </div>
      <div v-else class="empty-state">
        <div class="empty-icon">⚠️</div>
        <p class="empty-title">数据异常</p>
        <p class="empty-sub">有副线但没有 Main 主线。请联系开发者。</p>
      </div>

      <!-- 悬空副线（孤儿：parent_id 指向不存在的 storyline） -->
      <div v-if="orphanBranches.length" class="orphan-section">
        <h3 class="orphan-title">⚠️ 悬空副线（parent_id 指向不存在的 storyline）</h3>
        <div v-for="sl in orphanBranches" :key="sl.id" class="orphan-row">
          <span class="orphan-name">{{ sl.name }}</span>
          <span class="orphan-hint">需要修复或重新挂载</span>
        </div>
      </div>
    </div>

    <!-- Create/Edit Dialog -->
    <NeDialog v-model="showDialog" :title="editingId ? '编辑剧情线' : '新建剧情线'" size="md">
      <form @submit.prevent="handleSubmit" class="entity-form">
        <div class="form-group">
          <label class="form-label">名称 *</label>
          <input v-model="form.name" class="form-input" placeholder="剧情线名称" required />
        </div>
        <div class="form-group">
          <label class="form-label">描述</label>
          <textarea v-model="form.description" class="form-textarea" placeholder="剧情线描述" rows="3"></textarea>
        </div>
        <div class="form-grid">
          <div class="form-group">
            <label class="form-label">状态</label>
            <select v-model="form.status" class="form-select">
              <option value="Planned">计划中</option>
              <option value="Active">进行中</option>
              <option value="Resolved">已解决</option>
              <option value="Abandoned">已放弃</option>
            </select>
          </div>
          <div class="form-group">
            <label class="form-label">重要性</label>
            <select v-model="form.importance" class="form-select" :disabled="!!editingId && form.importance === 'Main'">
              <option value="Main">主线（每项目 1 条）</option>
              <option value="Important">重要支线</option>
              <option value="Normal">普通支线</option>
              <option value="Minor">次要支线</option>
            </select>
          </div>
          <div class="form-group">
            <label class="form-label">明/暗线</label>
            <select v-model="form.tone" class="form-select">
              <option value="light">明线（用户可见）</option>
              <option value="dark">暗线（伏笔/钩子）</option>
            </select>
          </div>
          <div class="form-group">
            <label class="form-label">可见性</label>
            <select v-model="form.visibility" class="form-select">
              <option value="visible">暴露给读者</option>
              <option value="hidden">隐藏（暗线用）</option>
            </select>
          </div>
        </div>
        <div class="form-group" v-if="form.importance !== 'Main'">
          <label class="form-label">挂载到（parent_id）</label>
          <select v-model="form.parent_id" class="form-select">
            <option value="">独立（不挂任何 storyline）</option>
            <option v-for="opt in parentOptions" :key="opt.id" :value="opt.id">
              {{ opt.label }}
            </option>
          </select>
        </div>
      </form>
      <template #footer>
        <button class="btn-secondary" @click="showDialog = false">取消</button>
        <button class="btn-primary" @click="handleSubmit">保存</button>
      </template>
    </NeDialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { useStoryStore } from '@/stores/story'
import type {
  Storyline,
  StorylineStatus,
  StorylineImportance,
  StorylineTone,
  StorylineVisibility,
} from '@/types/narrative'
import NeDialog from '@/components/ui/NeDialog.vue'
import SlNode from '@/components/storyline/SlNode.vue'
import SlBranch from '@/components/storyline/SlBranch.vue'

const route = useRoute()
const storyStore = useStoryStore()
const projectId = route.params.id as string

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
const toneMap: Record<StorylineTone, string> = {
  light: '明线',
  dark: '暗线',
}
const visibilityMap: Record<StorylineVisibility, string> = {
  visible: '可见',
  hidden: '隐藏',
}

function statusLabel(s: StorylineStatus) { return statusMap[s] ?? s }
function importanceLabel(i: StorylineImportance) { return importanceMap[i] ?? i }
function toneLabel(t: StorylineTone) { return toneMap[t] ?? t }
function visibilityLabel(v: StorylineVisibility) { return visibilityMap[v] ?? v }

// ===== 树形结构计算 =====
const mainLine = computed<Storyline | undefined>(() =>
  storyStore.storylines.find((s) => s.importance === 'Main'),
)

/** childrenMap: { parentId → child Storyline[] } */
const childrenMap = computed<Record<string, Storyline[]>>(() => {
  const map: Record<string, Storyline[]> = {}
  for (const rel of storyStore.storylineRelations) {
    if (!map[rel.parent_id]) map[rel.parent_id] = []
    const child = storyStore.storylines.find((s) => s.id === rel.child_id)
    if (child && !map[rel.parent_id].some((c) => c.id === child.id)) {
      map[rel.parent_id].push(child)
    }
  }
  return map
})

/** 悬空副线：parent_id 在 childrenMap 里找不到对应父 */
const orphanBranches = computed<Storyline[]>(() => {
  return storyStore.storylines.filter((s) => {
    if (s.importance === 'Main') return false
    const parent = storyStore.storylineRelations.find((r) => r.child_id === s.id)
    if (!parent) return true // 没挂载
    return !storyStore.storylines.find((p) => p.id === parent.parent_id)
  })
})

const branchCount = computed(() =>
  storyStore.storylines.filter((s) => s.importance !== 'Main').length,
)
const darkCount = computed(() =>
  storyStore.storylines.filter((s) => s.tone === 'dark').length,
)

/** 展开/折叠：默认全部展开；id → bool（true=展开, false=折叠） */
const expandedIds = ref<Record<string, boolean>>({})
function toggleExpand(id: string) {
  expandedIds.value[id] = expandedIds.value[id] === false
}
function expandAll() {
  const next: Record<string, boolean> = {}
  for (const sl of storyStore.storylines) next[sl.id] = true
  expandedIds.value = next
}
function collapseAll() {
  const next: Record<string, boolean> = {}
  for (const sl of storyStore.storylines) next[sl.id] = false
  expandedIds.value = next
}

// ===== 表单 =====
const showDialog = ref(false)
const editingId = ref<string | null>(null)
const form = ref({
  name: '',
  description: '',
  status: 'Planned' as StorylineStatus,
  importance: 'Normal' as StorylineImportance,
  tone: 'light' as StorylineTone,
  visibility: 'visible' as StorylineVisibility,
  parent_id: '',
  created_volume_id: '',
  resolved_volume_id: '',
})

const parentOptions = computed(() => {
  return storyStore.storylines
    .filter((s) => s.id !== editingId.value) // 不能挂到自己
    .map((s) => ({
      id: s.id,
      label: `${importanceLabel(s.importance)} · ${s.name}`,
    }))
})

function resetForm() {
  form.value = {
    name: '',
    description: '',
    status: 'Planned',
    importance: 'Normal',
    tone: 'light',
    visibility: 'visible',
    parent_id: '',
    created_volume_id: '',
    resolved_volume_id: '',
  }
}

function openCreate() {
  editingId.value = null
  resetForm()
  showDialog.value = true
}

function openCreateChild(parentId: string) {
  editingId.value = null
  resetForm()
  form.value.parent_id = parentId
  showDialog.value = true
}

function openEdit(sl: Storyline) {
  editingId.value = sl.id
  // 找当前挂载的 parent
  const rel = storyStore.storylineRelations.find((r) => r.child_id === sl.id)
  form.value = {
    name: sl.name,
    description: sl.description ?? '',
    status: sl.status,
    importance: sl.importance,
    tone: sl.tone ?? 'light',
    visibility: sl.visibility ?? 'visible',
    parent_id: rel?.parent_id ?? '',
    created_volume_id: sl.created_volume_id ?? '',
    resolved_volume_id: sl.resolved_volume_id ?? '',
  }
  showDialog.value = true
}

async function handleDelete(sl: Storyline) {
  if (!confirm(`确认删除剧情线「${sl.name}」？\n\n如该线挂有子副线，子副线会变成悬空（可手动修复）。`)) return
  await storyStore.deleteStoryline(sl.id)
}

async function handleSubmit() {
  if (!form.value.name.trim()) return
  const payload: any = {
    name: form.value.name.trim(),
    description: form.value.description.trim() || undefined,
    status: form.value.status,
    importance: form.value.importance,
    tone: form.value.tone,
    visibility: form.value.visibility,
    created_volume_id: form.value.created_volume_id.trim() || undefined,
    resolved_volume_id: form.value.resolved_volume_id.trim() || undefined,
  }
  // 副线（importance != Main）才传 parent_id；主线不允许挂载
  if (form.value.importance !== 'Main' && form.value.parent_id) {
    payload.parent_id = form.value.parent_id
  }
  if (editingId.value) {
    await storyStore.updateStoryline(editingId.value, payload)
  } else {
    await storyStore.createStoryline(projectId, payload)
  }
  showDialog.value = false
  editingId.value = null
  resetForm()
}

onMounted(async () => {
  await storyStore.fetchStorylines(projectId)
})
</script>

<style scoped>
.storylines-page {
  height: 100%;
  overflow-y: auto;
  padding: var(--space-6) var(--space-8);
  background: var(--bg-base);
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: var(--space-3);
  margin-bottom: var(--space-6);
}
.page-title {
  font-size: var(--text-2xl);
  font-weight: 700;
  font-family: var(--font-serif);
}
.header-actions {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: var(--space-2);
}
.header-hint {
  font-size: var(--text-sm);
  color: var(--text-tertiary);
  font-family: var(--font-serif);
  margin-right: var(--space-2);
}
.page-tip {
  margin: calc(var(--space-4) * -1) 0 var(--space-5);
  font-size: var(--text-xs);
  color: var(--text-tertiary);
}
.page-tip b {
  color: var(--text-secondary);
  font-weight: 600;
}
.btn-primary,
.btn-ghost {
  padding: var(--space-2) var(--space-4);
  border-radius: var(--radius-sm);
  font-size: var(--text-sm);
  font-family: inherit;
  cursor: pointer;
  transition: all var(--transition-fast);
}
.btn-primary {
  background: var(--color-primary);
  border: 1px solid var(--color-primary);
  color: white;
}
.btn-primary:hover { background: var(--color-primary-hover); border-color: var(--color-primary-hover); }
.btn-ghost {
  background: transparent;
  border: 1px solid var(--border-default);
  color: var(--text-secondary);
}
.btn-ghost:hover { background: var(--bg-hover); border-color: var(--border-emphasis); color: var(--text-primary); }
.btn-secondary {
  padding: var(--space-2) var(--space-4);
  background: transparent;
  border: 1px solid var(--border-default);
  color: var(--text-secondary);
  border-radius: var(--radius-sm);
  font-size: var(--text-sm);
  font-family: inherit;
  cursor: pointer;
}
.btn-secondary:hover { background: var(--bg-hover); }

/* Create/Edit form — 之前缺失导致弹窗内部没有样式 */
.entity-form {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}
.form-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--space-4);
}
.form-group {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  min-width: 0;
}
.form-label {
  font-size: var(--text-sm);
  font-weight: 500;
  color: var(--text-secondary);
}
.form-input,
.form-textarea,
.form-select {
  width: 100%;
  padding: var(--space-2) var(--space-3);
  background: var(--bg-base);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: var(--text-sm);
  font-family: inherit;
  outline: none;
  transition: border-color var(--transition-fast), box-shadow var(--transition-fast);
}
.form-input:focus,
.form-textarea:focus,
.form-select:focus {
  border-color: var(--color-primary);
  box-shadow: 0 0 0 2px var(--color-primary-subtle);
}
.form-textarea {
  resize: vertical;
  min-height: 76px;
  line-height: var(--leading-relaxed);
}
.form-select:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.error-banner {
  background: var(--color-error-subtle);
  color: var(--color-error);
  border: 1px solid var(--color-error);
  border-radius: var(--radius-sm);
  padding: var(--space-3);
  margin-bottom: var(--space-4);
  font-size: var(--text-sm);
}

.empty-state {
  text-align: center;
  padding: var(--space-12) var(--space-4);
  color: var(--text-tertiary);
}
.empty-icon { font-size: 64px; margin-bottom: var(--space-4); opacity: 0.6; }
.empty-title { font-size: var(--text-lg); color: var(--text-primary); margin-bottom: var(--space-2); font-family: var(--font-serif); }
.empty-sub { font-size: var(--text-sm); margin-bottom: var(--space-6); }

.storyline-tree { display: flex; flex-direction: column; gap: 0; }
.tree-root { display: flex; flex-direction: column; }
.tree-branches { display: flex; flex-direction: column; }

/* 主线根的"还没有副线"提示 */
.hint-no-branches {
  margin-left: 56px;
  margin-bottom: var(--space-3);
  font-size: var(--text-sm);
  color: var(--text-tertiary);
  font-style: italic;
}
.link {
  color: var(--color-primary);
  cursor: pointer;
  text-decoration: underline;
}

/* 悬空副线区 */
.orphan-section {
  margin-top: var(--space-6);
  padding: var(--space-3) var(--space-4);
  background: var(--color-error-subtle);
  border: 1px dashed var(--color-error);
  border-radius: var(--radius-md);
}
.orphan-title {
  font-size: var(--text-sm);
  color: var(--color-error);
  margin: 0 0 var(--space-2);
  font-weight: 600;
}
.orphan-row {
  display: flex;
  gap: var(--space-3);
  font-size: var(--text-sm);
  padding: 2px 0;
}
.orphan-name { color: var(--text-primary); font-weight: 500; }
.orphan-hint { color: var(--text-tertiary); font-size: var(--text-xs); }
</style>
