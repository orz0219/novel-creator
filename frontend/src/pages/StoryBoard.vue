<template>
  <div class="board-page">
    <!-- 工具条：视图切换 / 筛选 / 搜索 / 新建 -->
    <header class="board-toolbar">
      <div class="toolbar-heading">
        <h1 class="page-title">故事进度</h1>
        <p class="page-subtitle">
          <template v-if="view === 'status'">拖动章节卡片改变写作状态 · 点卷/弧前的圆点改它们的状态</template>
          <template v-else>按卷总览每卷进度 · 要拖动改状态请切到「按状态」</template>
        </p>
      </div>

      <div class="toolbar-controls">
        <div class="view-switch">
          <button class="view-btn" :class="{ active: view === 'status' }" @click="view = 'status'">按状态</button>
          <button class="view-btn" :class="{ active: view === 'volume' }" @click="view = 'volume'">按卷</button>
        </div>

        <select v-if="view === 'status'" v-model="volumeFilter" class="toolbar-select">
          <option value="">全部卷</option>
          <option v-for="vol in volumeRoots" :key="vol.id" :value="vol.id">{{ vol.title }}</option>
        </select>

        <div class="search-box">
          <Search :size="13" class="search-icon" />
          <input v-model="keyword" class="search-input" placeholder="搜索标题" />
        </div>

        <button class="btn-primary" @click="openCreateDialog">
          <Plus :size="14" />
          <span>新建</span>
        </button>
      </div>
    </header>

    <!-- 错误不再被吞成「暂无节点」 -->
    <div v-if="storyStore.error" class="error-banner">
      <span class="error-text">{{ storyStore.error }}</span>
      <button class="error-retry" @click="reload">重试</button>
    </div>

    <div v-if="storyStore.loading" class="state-block">
      <Loader2 class="spin" :size="18" />
      <span class="state-text">加载中…</span>
    </div>

    <div v-else-if="!storyStore.nodes.length" class="empty-state">
      <BookOpen class="empty-icon" :size="40" />
      <span class="empty-title">这个故事还没有结构</span>
      <span class="empty-desc">先建「卷 → 弧 → 章」，之后这一页就是你每天的写作进度板</span>
      <button class="btn-primary" @click="openCreateDialog">＋ 新建第一个节点</button>
    </div>

    <!-- 按状态：列 = 写作状态，卡片 = 章节/场景，卷与弧是分组头 -->
    <div v-else-if="view === 'status'" class="board-columns">
      <section
        v-for="col in statusColumns"
        :key="col.status"
        class="board-column"
        :class="{ collapsed: col.collapsed, 'drop-active': dragOverStatus === col.status }"
        @dragover.prevent="onDragOverColumn(col.status)"
        @dragleave="onDragLeaveColumn(col.status, $event)"
        @drop.prevent="onDropColumn(col.status)"
      >
        <header class="column-header" @click="toggleColumn(col.status, col.collapsed)">
          <span class="column-dot" :class="col.status"></span>
          <span class="column-title">{{ col.label }}</span>
          <span class="column-count">{{ col.rows.length }}</span>
        </header>

        <p class="column-hint">{{ col.hint }}</p>

        <div class="column-body">
          <div v-if="!col.rows.length" class="column-empty">把卡片拖到这里</div>

          <template v-for="row in col.rows" :key="row.node.id">
            <!-- 卷 / 弧：分组头，不占卡片位置，但状态依旧可改 -->
            <div
              v-if="isGroupNode(row.node)"
              class="group-head"
              :class="[`st-${row.node.status}`, { 'group-collapsed': collapsedGroups.has(row.node.id) }]"
              :style="{ marginLeft: indent(row.depth) }"
            >
              <button class="group-toggle" @click="toggleGroup(row.node.id)">
                <component :is="collapsedGroups.has(row.node.id) ? ChevronRight : ChevronDown" :size="12" />
              </button>

              <NeDropdown
                :items="statusMenuItems"
                align="left"
                @select="onGroupStatusSelect(row.node, $event)"
              >
                <template #trigger>
                  <span class="status-dot clickable" :class="row.node.status" :title="`改「${row.node.title}」的状态`"></span>
                </template>
              </NeDropdown>

              <span class="group-type">{{ typeLabel(row.node) }}</span>
              <span class="group-title">{{ row.node.title }}</span>
              <span class="group-count">{{ descendantChapterCount(row.node) }} 章</span>

              <button class="group-action" title="在故事结构里查看" @click="goToStructure">
                <ArrowUpRight :size="12" />
              </button>
            </div>

            <!-- 章 / 场景：卡片，可拖动改状态 -->
            <article
              v-else
              class="board-card"
              :class="[`st-${row.node.status}`, { dragging: draggingId === row.node.id }]"
              :style="{ marginLeft: indent(row.depth) }"
              :draggable="true"
              @dragstart="onCardDragStart(row.node, $event)"
              @dragend="onCardDragEnd"
            >
              <div class="card-main" @click="goToWrite(row.node)">
                <div class="card-title">{{ row.node.title }}</div>
                <div class="card-meta">
                  <span class="card-type">{{ typeLabel(row.node) }}</span>
                  <span class="card-crumb" v-if="crumb(row)">{{ crumb(row) }}</span>
                  <span class="card-words">{{ formatWords(wordCount(row.node)) }}</span>
                </div>
              </div>

              <div class="card-actions">
                <button
                  v-if="row.node.status !== 'Completed'"
                  class="card-btn"
                  title="标记为已完成"
                  @click.stop="applyStatus(row.node, 'Completed')"
                >
                  <Check :size="12" />
                </button>
                <button class="card-btn" title="打开写作页" @click.stop="goToWrite(row.node)">
                  <ArrowUpRight :size="12" />
                </button>
              </div>
            </article>
          </template>
        </div>
      </section>
    </div>

    <!-- 按卷：列 = 卷，卡片 = 卷下的章节，颜色 = 状态 -->
    <div v-else class="board-columns">
      <section v-for="col in rootColumns" :key="col.root.id" class="board-column volume-column">
        <header class="column-header static">
          <span class="column-dot" :class="col.root.status"></span>
          <span class="column-title">{{ col.root.title }}</span>
        </header>

        <div class="volume-progress">
          <div class="progress-track">
            <div class="progress-fill" :style="{ width: col.percent + '%' }"></div>
          </div>
          <span class="progress-text">{{ col.chapterDone }} / {{ col.chapterTotal }} 章已完成</span>
        </div>

        <div class="column-body">
          <div v-if="!col.rows.length" class="column-empty">这一卷还没有下级节点</div>

          <template v-for="row in col.rows" :key="row.node.id">
            <div
              v-if="isGroupNode(row.node)"
              class="group-head"
              :class="[`st-${row.node.status}`, { 'group-collapsed': collapsedGroups.has(row.node.id) }]"
              :style="{ marginLeft: indent(row.depth) }"
            >
              <button class="group-toggle" @click="toggleGroup(row.node.id)">
                <component :is="collapsedGroups.has(row.node.id) ? ChevronRight : ChevronDown" :size="12" />
              </button>
              <span class="status-dot" :class="row.node.status"></span>
              <span class="group-type">{{ typeLabel(row.node) }}</span>
              <span class="group-title">{{ row.node.title }}</span>
              <span class="group-count">{{ descendantChapterCount(row.node) }} 章</span>
            </div>

            <article v-else class="board-card" :class="`st-${row.node.status}`" :style="{ marginLeft: indent(row.depth) }">
              <div class="card-main" @click="goToWrite(row.node)">
                <div class="card-title">{{ row.node.title }}</div>
                <div class="card-meta">
                  <span class="card-type">{{ typeLabel(row.node) }}</span>
                  <span class="card-words">{{ formatWords(wordCount(row.node)) }}</span>
                  <span class="card-status-text" :class="row.node.status">{{ STATUS_LABELS[row.node.status] }}</span>
                </div>
              </div>
              <div class="card-actions">
                <button class="card-btn" title="打开写作页" @click.stop="goToWrite(row.node)">
                  <ArrowUpRight :size="12" />
                </button>
              </div>
            </article>
          </template>
        </div>
      </section>
    </div>

    <!-- 新建节点 -->
    <NeDialog v-model="showCreateDialog" title="新建节点" size="sm">
      <div class="form-group">
        <label class="form-label">类型</label>
        <select v-model="createForm.node_type" class="form-select">
          <option value="Volume">卷</option>
          <option value="Arc">弧</option>
          <option value="Chapter">章</option>
          <option value="Scene">场景</option>
        </select>
      </div>

      <div class="form-group" v-if="parentOptions.length">
        <label class="form-label">上级</label>
        <select v-model="createForm.parent_id" class="form-select">
          <option value="">请选择…</option>
          <option v-for="opt in parentOptions" :key="opt.value" :value="opt.value">{{ opt.label }}</option>
        </select>
      </div>

      <div class="form-group">
        <label class="form-label">标题</label>
        <input v-model="createForm.title" class="form-input" placeholder="例如：第 13 章 · 第一次收租" />
      </div>

      <p class="form-error" v-if="createError">{{ createError }}</p>

      <template #footer>
        <button class="btn-secondary" @click="showCreateDialog = false">取消</button>
        <button class="btn-primary" :disabled="!!createError || creating" @click="handleCreate">
          {{ creating ? '创建中…' : '创建' }}
        </button>
      </template>
    </NeDialog>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ArrowUpRight, BookOpen, Check, ChevronDown, ChevronRight, Loader2, Plus, Search } from 'lucide-vue-next'
import { useStoryStore } from '@/stores/story'
import { useUiStore } from '@/stores/ui'
import NeDialog from '@/components/ui/NeDialog.vue'
import NeDropdown from '@/components/ui/NeDropdown.vue'
import type { NarrativeNode, NarrativeNodeStatus, NarrativeNodeType, TreeNode } from '@/types'

const route = useRoute()
const router = useRouter()
const storyStore = useStoryStore()
const ui = useUiStore()
const projectId = route.params.id as string

const STATUS_ORDER: NarrativeNodeStatus[] = ['Draft', 'Planned', 'InProgress', 'Completed', 'Archived']
const STATUS_LABELS: Record<NarrativeNodeStatus, string> = {
  Draft: '草稿',
  Planned: '已规划',
  InProgress: '进行中',
  Completed: '已完成',
  Archived: '已归档',
}
const STATUS_HINTS: Record<NarrativeNodeStatus, string> = {
  Draft: '还没动笔',
  Planned: '定好大纲，等着写',
  InProgress: '正在写',
  Completed: '定稿',
  Archived: '废弃或暂缓',
}
const statusMenuItems = STATUS_ORDER.map((s) => ({ id: s, label: STATUS_LABELS[s] }))

/** 列里的一行：DFS 序 + 层级深度 + 祖先链（面包屑与折叠过滤都要用） */
interface BoardRow {
  node: NarrativeNode
  depth: number
  ancestors: NarrativeNode[]
}

const view = ref<'status' | 'volume'>('status')
const keyword = ref('')
const volumeFilter = ref('')
const collapsedGroups = ref(new Set<string>())
const manualColumnCollapse = ref<Partial<Record<NarrativeNodeStatus, boolean>>>({})

const draggingId = ref<string | null>(null)
const dragOverStatus = ref<NarrativeNodeStatus | null>(null)

// ---- 树展开 ----

function flattenTree(list: TreeNode[], ancestors: NarrativeNode[] = [], out: BoardRow[] = []): BoardRow[] {
  for (const item of list) {
    out.push({ node: item, depth: ancestors.length, ancestors })
    flattenTree(item.children, [...ancestors, item], out)
  }
  return out
}

/** 关键词过滤：保留命中节点 + 它的整条祖先链，否则缩进层级会断掉 */
function pruneTree(list: TreeNode[], kw: string): TreeNode[] {
  const kept: TreeNode[] = []
  for (const node of list) {
    const keptChildren = pruneTree(node.children, kw)
    if (node.title.toLowerCase().includes(kw) || keptChildren.length) {
      kept.push({ ...node, children: keptChildren })
    }
  }
  return kept
}

const filteredTree = computed<TreeNode[]>(() => {
  const base = volumeFilter.value
    ? storyStore.tree.filter((v) => v.id === volumeFilter.value)
    : storyStore.tree
  const kw = keyword.value.trim().toLowerCase()
  return kw ? pruneTree(base, kw) : base
})

const volumeRoots = computed(() => storyStore.tree.filter((n) => n.node_type === 'Volume'))

/** 折叠的分组：组内所有后代行都不渲染 */
function visibleRows(rows: BoardRow[]): BoardRow[] {
  if (!collapsedGroups.value.size) return rows
  return rows.filter((r) => !r.ancestors.some((a) => collapsedGroups.value.has(a.id)))
}

// ---- 按状态视图 ----

const statusColumns = computed(() => {
  const rows = visibleRows(flattenTree(filteredTree.value))
  return STATUS_ORDER.map((status) => {
    const colRows = rows.filter((r) => r.node.status === status)
    const isManual = manualColumnCollapse.value[status]
    return {
      status,
      label: STATUS_LABELS[status],
      hint: STATUS_HINTS[status],
      rows: colRows,
      // 空列默认折成窄条；拖拽悬停时临时展开，否则没法当放置目标
      collapsed: (isManual ?? colRows.length === 0) && dragOverStatus.value !== status,
    }
  })
})

// ---- 按卷视图 ----

const rootColumns = computed(() => {
  const kw = keyword.value.trim().toLowerCase()
  const roots = kw ? pruneTree(storyStore.tree, kw) : storyStore.tree
  return roots.map((root) => {
    const rows = visibleRows(
      flattenTree(root.children, [root]),
    )
    const chapters = rows.filter((r) => r.node.node_type === 'Chapter')
    const done = chapters.filter((r) => r.node.status === 'Completed').length
    return {
      root: root as NarrativeNode,
      rows,
      chapterTotal: chapters.length,
      chapterDone: done,
      percent: chapters.length ? Math.round((done / chapters.length) * 100) : 0,
    }
  })
})

// ---- 展示辅助 ----

function isGroupNode(node: NarrativeNode): boolean {
  return node.node_type === 'Volume' || node.node_type === 'Arc'
}

function typeLabel(node: NarrativeNode): string {
  const map: Record<string, string> = { Volume: '卷', Arc: '弧', Chapter: '章', Scene: '场', Sequence: '序列', Beat: '拍' }
  return map[node.node_type] ?? node.node_type.replace('custom:', '')
}

function indent(depth: number): string {
  return `${depth * 14}px`
}

function crumb(row: BoardRow): string {
  const named = row.ancestors.filter((a) => a.node_type === 'Volume' || a.node_type === 'Arc')
  if (!named.length) return ''
  return named.map((a) => a.title).join(' › ')
}

function wordCount(node: NarrativeNode): number {
  // 与写作页的字数口径保持一致：去掉空白后的字符数
  return (node.content || '').replace(/\s/g, '').length
}

function formatWords(count: number): string {
  if (!count) return '未动笔'
  return count >= 1000 ? `${(count / 1000).toFixed(1)}k 字` : `${count} 字`
}

function descendantChapterCount(node: NarrativeNode): number {
  return chapterCountMap.value.get(node.id) ?? 0
}

/** 每个节点的后代章节数：一次 DFS 全部算好，不在渲染时反复重建子树 */
const chapterCountMap = computed(() => {
  const map = new Map<string, number>()
  const walk = (list: TreeNode[]): number => {
    let total = 0
    for (const node of list) {
      const below = walk(node.children)
      const self = node.node_type === 'Chapter' ? 1 : 0
      map.set(node.id, below + self)
      total += below + self
    }
    return total
  }
  walk(storyStore.tree)
  return map
})

// ---- 折叠 ----

function toggleGroup(id: string) {
  const next = new Set(collapsedGroups.value)
  if (next.has(id)) next.delete(id)
  else next.add(id)
  collapsedGroups.value = next
}

function toggleColumn(status: NarrativeNodeStatus, currentlyCollapsed: boolean) {
  manualColumnCollapse.value = { ...manualColumnCollapse.value, [status]: !currentlyCollapsed }
}

// ---- 拖拽改状态 ----

function onCardDragStart(node: NarrativeNode, event: DragEvent) {
  draggingId.value = node.id
  // Firefox 必须调用 setData 才会真正启动拖拽（Chrome 不调用也能拖）
  event.dataTransfer!.effectAllowed = 'move'
  event.dataTransfer!.setData('text/plain', node.id)
}

function onCardDragEnd() {
  draggingId.value = null
  dragOverStatus.value = null
}

function onDragOverColumn(status: NarrativeNodeStatus) {
  if (!draggingId.value) return
  dragOverStatus.value = status
}

function onDragLeaveColumn(status: NarrativeNodeStatus, event: DragEvent) {
  if (dragOverStatus.value !== status) return
  // 在列内部子元素之间移动也会触发 dragleave，这种不算离开
  const related = event.relatedTarget as Node | null
  if (related && (event.currentTarget as HTMLElement).contains(related)) return
  dragOverStatus.value = null
}

async function onDropColumn(status: NarrativeNodeStatus) {
  const id = draggingId.value
  draggingId.value = null
  dragOverStatus.value = null
  // 从窗口外拖进来的文件/文本不是我们的卡片，直接忽略（它们不携带 draggingId）
  if (!id) return

  const node = storyStore.nodes.find((n) => n.id === id)
  if (!node) throw new Error(`拖拽的节点已不在当前列表中：${id}`)
  if (node.status === status) return

  await applyStatus(node, status)
}

/** 组头的状态下拉：选中的 id 就是目标状态 */
function onGroupStatusSelect(node: NarrativeNode, item: { id: string }) {
  void applyStatus(node, item.id as NarrativeNodeStatus)
}

async function applyStatus(node: NarrativeNode, status: NarrativeNodeStatus) {
  try {
    await storyStore.setNodeStatus(node.id, status)
    ui.addToast({ type: 'success', title: '状态已更新', message: `「${node.title}」→ ${STATUS_LABELS[status]}` })
  } catch (e) {
    ui.addToast({
      type: 'error',
      title: '改状态失败',
      message: e instanceof Error ? e.message : String(e),
    })
  }
}

// ---- 跳转 ----

function goToWrite(node: NarrativeNode) {
  if (node.node_type !== 'Chapter' && node.node_type !== 'Scene') return
  router.push(`/project/${projectId}/write/${node.id}`)
}

function goToStructure() {
  router.push(`/project/${projectId}/story`)
}

async function reload() {
  await storyStore.fetchNodes(projectId)
}

// ---- 新建 ----

const showCreateDialog = ref(false)
const creating = ref(false)
const createForm = ref<{ node_type: NarrativeNodeType; parent_id: string; title: string }>({
  node_type: 'Chapter',
  parent_id: '',
  title: '',
})

const PARENT_TYPE: Partial<Record<NarrativeNodeType, NarrativeNodeType>> = {
  Arc: 'Volume',
  Chapter: 'Arc',
  Scene: 'Chapter',
}

const parentOptions = computed(() => {
  const parentType = PARENT_TYPE[createForm.value.node_type]
  if (!parentType) return []
  return storyStore.nodes
    .filter((n) => n.node_type === parentType)
    .map((n) => ({ value: n.id, label: n.title }))
})

const createError = computed(() => {
  if (!createForm.value.title.trim()) return '请填写标题'
  const parentType = PARENT_TYPE[createForm.value.node_type]
  if (!parentType) return ''
  if (!parentOptions.value.length) {
    const names: Partial<Record<NarrativeNodeType, string>> = { Volume: '卷', Arc: '弧', Chapter: '章' }
    return `还没有${names[parentType]}，请先创建一个`
  }
  if (!createForm.value.parent_id) return '请选择上级节点'
  return ''
})

function openCreateDialog() {
  const hasVolume = storyStore.nodes.some((n) => n.node_type === 'Volume')
  createForm.value = {
    node_type: hasVolume ? 'Chapter' : 'Volume',
    parent_id: '',
    title: '',
  }
  showCreateDialog.value = true
}

async function handleCreate() {
  if (createError.value) return
  creating.value = true
  const title = createForm.value.title.trim()
  try {
    await storyStore.createNode(projectId, {
      node_type: createForm.value.node_type,
      title,
      parent_id: createForm.value.parent_id || undefined,
    } as Partial<NarrativeNode>)
    showCreateDialog.value = false
    ui.addToast({ type: 'success', title: '已创建', message: title })
  } catch (e) {
    ui.addToast({
      type: 'error',
      title: '创建失败',
      message: e instanceof Error ? e.message : String(e),
    })
  } finally {
    creating.value = false
  }
}

onMounted(async () => {
  await storyStore.fetchNodes(projectId)
})
</script>

<style scoped>
.board-page { height: 100%; display: flex; flex-direction: column; overflow: hidden; }

/* ---- 工具条 ---- */
.board-toolbar {
  display: flex; align-items: flex-end; justify-content: space-between; gap: var(--space-4);
  padding: var(--space-4) var(--space-6) var(--space-3);
  border-bottom: 1px solid var(--border-default);
  flex-shrink: 0;
}
.toolbar-heading { min-width: 0; }
.page-title { font-size: var(--text-2xl); font-weight: 700; font-family: var(--font-serif); }
.page-subtitle { font-size: var(--text-xs); color: var(--text-tertiary); margin-top: var(--space-1); }
.toolbar-controls { display: flex; align-items: center; gap: var(--space-2); flex-shrink: 0; }

.view-switch { display: inline-flex; border: 1px solid var(--border-default); border-radius: var(--radius-sm); overflow: hidden; }
.view-btn {
  padding: var(--space-1) var(--space-3); background: transparent; border: none;
  color: var(--text-tertiary); font-size: var(--text-xs); font-family: inherit; cursor: pointer;
  transition: all var(--transition-fast);
}
.view-btn:hover { background: var(--bg-hover); color: var(--text-primary); }
.view-btn.active { background: var(--bg-selected); color: var(--text-primary); }

.toolbar-select {
  padding: var(--space-1) var(--space-2); background: var(--bg-base);
  border: 1px solid var(--border-default); border-radius: var(--radius-sm);
  color: var(--text-secondary); font-size: var(--text-xs); font-family: inherit;
  outline: none; cursor: pointer; max-width: 160px;
}
.toolbar-select:focus { border-color: var(--color-primary); }

.search-box {
  display: flex; align-items: center; gap: var(--space-2);
  padding: var(--space-1) var(--space-2);
  background: var(--bg-base); border: 1px solid var(--border-default); border-radius: var(--radius-sm);
}
.search-box:focus-within { border-color: var(--color-primary); }
.search-icon { color: var(--text-tertiary); flex-shrink: 0; }
.search-input {
  width: 130px; background: transparent; border: none; outline: none;
  color: var(--text-primary); font-size: var(--text-xs); font-family: inherit;
}
.search-input::placeholder { color: var(--text-disabled); }

.btn-primary {
  display: inline-flex; align-items: center; gap: var(--space-1);
  padding: var(--space-2) var(--space-3); background: var(--color-primary); border: none;
  color: white; border-radius: var(--radius-sm); font-size: var(--text-xs);
  font-family: inherit; cursor: pointer; transition: background var(--transition-fast);
}
.btn-primary:hover:not(:disabled) { background: var(--color-primary-hover); }
.btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }
.btn-secondary {
  padding: var(--space-2) var(--space-3); background: transparent;
  border: 1px solid var(--border-default); color: var(--text-secondary);
  border-radius: var(--radius-sm); font-size: var(--text-xs); font-family: inherit; cursor: pointer;
}
.btn-secondary:hover { background: var(--bg-hover); color: var(--text-primary); }

/* ---- 状态块 ---- */
.error-banner {
  display: flex; align-items: center; gap: var(--space-3);
  margin: var(--space-3) var(--space-6) 0; padding: var(--space-3) var(--space-4);
  background: var(--color-error-subtle); border-radius: var(--radius-sm); font-size: var(--text-sm);
}
.error-text { color: var(--color-error); flex: 1; }
.error-retry {
  padding: 2px 10px; background: transparent; border: 1px solid var(--color-error);
  color: var(--color-error); border-radius: var(--radius-sm); font-size: var(--text-xs);
  font-family: inherit; cursor: pointer;
}
.state-block { flex: 1; display: flex; align-items: center; justify-content: center; gap: var(--space-2); color: var(--text-tertiary); }
.state-text { font-size: var(--text-sm); }
.spin { animation: spin 0.9s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }

.empty-state {
  flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center;
  gap: var(--space-2); color: var(--text-tertiary); padding: var(--space-12);
}
.empty-icon { color: var(--text-disabled); margin-bottom: var(--space-2); }
.empty-title { font-size: var(--text-md); color: var(--text-secondary); }
.empty-desc { font-size: var(--text-xs); margin-bottom: var(--space-3); }

/* ---- 列 ---- */
.board-columns {
  flex: 1; display: flex; align-items: stretch; gap: var(--space-3);
  padding: var(--space-4) var(--space-6); overflow-x: auto; overflow-y: hidden;
}
.board-column {
  display: flex; flex-direction: column;
  flex: 1 1 0; min-width: 290px;
  background: var(--bg-panel); border: 1px solid var(--border-default);
  border-radius: var(--radius-md); overflow: hidden;
  transition: flex-basis var(--transition-normal), min-width var(--transition-normal), border-color var(--transition-fast), background var(--transition-fast);
}
.board-column.collapsed { flex: 0 0 46px; min-width: 46px; cursor: pointer; }
.board-column.collapsed .column-hint,
.board-column.collapsed .column-body { display: none; }
.board-column.collapsed .column-header {
  flex-direction: row-reverse; height: 100%; padding: var(--space-4) 0;
  writing-mode: vertical-rl; gap: var(--space-3);
}
.board-column.drop-active { border-color: var(--color-primary); background: var(--color-primary-subtle); }

.column-header {
  display: flex; align-items: center; gap: var(--space-2);
  padding: var(--space-3) var(--space-4);
  border-bottom: 1px solid var(--border-muted);
  cursor: pointer; flex-shrink: 0;
}
.column-header.static { cursor: default; }
.column-header:hover { background: var(--bg-hover); }
.column-header.static:hover { background: transparent; }
.column-title { font-size: var(--text-sm); font-weight: 600; }
.column-count {
  margin-left: auto; font-size: var(--text-xs); color: var(--text-tertiary);
  background: var(--bg-panel-secondary); padding: 1px 7px; border-radius: 10px;
}
.column-dot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; }

.column-hint {
  padding: var(--space-2) var(--space-4) 0;
  font-size: 10px; color: var(--text-disabled);
}
.column-body { flex: 1; padding: var(--space-2); overflow-y: auto; }
.column-empty {
  margin: var(--space-2); padding: var(--space-4) var(--space-3);
  border: 1px dashed var(--border-default); border-radius: var(--radius-sm);
  text-align: center; font-size: var(--text-xs); color: var(--text-disabled);
}

/* ---- 分组头（卷 / 弧） ---- */
.group-head {
  display: flex; align-items: center; gap: var(--space-2);
  padding: var(--space-1) var(--space-2);
  margin-bottom: var(--space-1);
  border-left: 2px solid var(--border-emphasis);
  background: var(--bg-panel-secondary);
  border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
}
.group-head.st-Draft { border-left-color: var(--color-warning); }
.group-head.st-Planned { border-left-color: var(--text-tertiary); }
.group-head.st-InProgress { border-left-color: var(--color-accent); }
.group-head.st-Completed { border-left-color: var(--color-success); }
.group-head.st-Archived { border-left-color: var(--border-emphasis); }
.group-head.group-collapsed { opacity: 0.65; }
.group-toggle {
  display: inline-flex; align-items: center; justify-content: center;
  width: 16px; height: 16px; padding: 0; background: transparent; border: none;
  color: var(--text-tertiary); cursor: pointer; flex-shrink: 0;
}
.group-toggle:hover { color: var(--text-primary); }
.group-type {
  font-size: 10px; color: var(--text-tertiary); flex-shrink: 0;
  border: 1px solid var(--border-muted); border-radius: 2px; padding: 0 3px;
}
.group-title {
  font-size: var(--text-xs); color: var(--text-secondary); font-weight: 500;
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
.group-count { margin-left: auto; font-size: 10px; color: var(--text-disabled); flex-shrink: 0; }
.group-action {
  display: inline-flex; align-items: center; padding: 2px; background: transparent;
  border: none; color: var(--text-disabled); cursor: pointer; flex-shrink: 0;
}
.group-action:hover { color: var(--color-primary-text); }

/* ---- 卡片（章 / 场景） ---- */
.board-card {
  display: flex; align-items: center; gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  margin-bottom: var(--space-2);
  background: var(--bg-panel-secondary);
  border: 1px solid var(--border-muted); border-left: 2px solid var(--border-emphasis);
  border-radius: var(--radius-sm);
  cursor: grab; transition: border-color var(--transition-fast), background var(--transition-fast);
}
.board-card:hover { border-color: var(--border-emphasis); background: var(--bg-hover); }
.board-card.st-Draft { border-left-color: var(--color-warning); }
.board-card.st-Planned { border-left-color: var(--text-tertiary); }
.board-card.st-InProgress { border-left-color: var(--color-accent); }
.board-card.st-Completed { border-left-color: var(--color-success); }
.board-card.st-Archived { border-left-color: var(--border-emphasis); }
.board-card.dragging { opacity: 0.4; cursor: grabbing; }

.card-main { flex: 1; min-width: 0; cursor: pointer; }
.card-title {
  font-size: var(--text-sm); color: var(--text-primary);
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
.card-meta { display: flex; align-items: center; gap: var(--space-2); margin-top: 2px; }
.card-type { font-size: 10px; color: var(--text-tertiary); }
.card-crumb {
  font-size: 10px; color: var(--text-disabled);
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
.card-words { font-size: 10px; color: var(--text-disabled); margin-left: auto; flex-shrink: 0; }
.card-status-text { font-size: 10px; flex-shrink: 0; }
.card-status-text.Completed { color: var(--color-success); }
.card-status-text.InProgress { color: var(--color-accent); }
.card-status-text.Draft { color: var(--color-warning); }
.card-status-text.Planned { color: var(--text-tertiary); }
.card-status-text.Archived { color: var(--text-disabled); }

.card-actions { display: flex; gap: 2px; flex-shrink: 0; opacity: 0; transition: opacity var(--transition-fast); }
.board-card:hover .card-actions { opacity: 1; }
.card-btn {
  display: inline-flex; align-items: center; justify-content: center;
  width: 20px; height: 20px; padding: 0; background: transparent;
  border: 1px solid var(--border-default); border-radius: 3px;
  color: var(--text-tertiary); cursor: pointer;
}
.card-btn:hover { border-color: var(--color-primary); color: var(--color-primary-text); }

/* ---- 状态色点 ---- */
.status-dot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; display: inline-block; }
.status-dot.clickable { cursor: pointer; }
.status-dot.clickable:hover { box-shadow: 0 0 0 3px var(--bg-selected); }
.status-dot.Draft, .column-dot.Draft { background: var(--color-warning); }
.status-dot.Planned, .column-dot.Planned { background: var(--text-tertiary); }
.status-dot.InProgress, .column-dot.InProgress { background: var(--color-accent); }
.status-dot.Completed, .column-dot.Completed { background: var(--color-success); }
.status-dot.Archived, .column-dot.Archived { background: var(--border-emphasis); }

/* ---- 按卷视图 ---- */
.volume-progress { padding: var(--space-2) var(--space-4); border-bottom: 1px solid var(--border-muted); }
.progress-track { height: 3px; background: var(--bg-panel-tertiary); border-radius: 2px; overflow: hidden; }
.progress-fill { height: 100%; background: var(--color-success); transition: width var(--transition-normal); }
.progress-text { display: block; margin-top: var(--space-1); font-size: 10px; color: var(--text-tertiary); }

/* ---- 表单 ---- */
.form-group { margin-bottom: var(--space-3); }
.form-label { display: block; font-size: var(--text-xs); color: var(--text-secondary); margin-bottom: var(--space-1); }
.form-select, .form-input {
  width: 100%; padding: var(--space-2) var(--space-3);
  background: var(--bg-base); border: 1px solid var(--border-default);
  border-radius: var(--radius-sm); color: var(--text-primary);
  font-size: var(--text-sm); font-family: inherit; outline: none;
}
.form-select:focus, .form-input:focus { border-color: var(--color-primary); }
.form-error { font-size: var(--text-xs); color: var(--color-warning); }
</style>
