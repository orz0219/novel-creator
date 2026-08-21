<template>
  <div class="story-page">
    <div class="page-header">
      <h1 class="page-title">故事结构</h1>
      <button class="btn-primary" @click="showCreateDialog = true">+ 新建卷/章节</button>
    </div>

    <div v-if="storyStore.error" class="error-banner">{{ storyStore.error }}</div>

    <div v-if="storyStore.loading" class="loading-state">
      <span class="loading-text">加载中…</span>
    </div>

    <div v-else-if="!storyStore.tree.length" class="empty-state">
      <span class="empty-icon">📖</span>
      <span class="empty-text">暂无故事结构，点击上方按钮创建</span>
    </div>

    <div v-else class="story-tree">
      <template v-for="vol in storyStore.tree" :key="vol.id">
        <div class="tree-volume">
          <div class="volume-header" @click="toggleExpand(vol.id)">
            <span class="expand-icon">{{ expanded[vol.id] ? '▼' : '▶' }}</span>
            <span class="volume-title">{{ vol.title }}</span>
            <StatusBadge :status="(vol.status || '').toLowerCase()" :label="vol.status" />
            <span class="volume-meta">{{ vol.description }}</span>
            <button class="action-btn" @click.stop="openEdit(vol)">编辑</button>
            <button class="action-btn danger" @click.stop="handleDelete(vol)">删除</button>
          </div>
          <div class="volume-body" v-if="expanded[vol.id]">
            <template v-for="arc in vol.children" :key="arc.id">
              <div class="tree-arc">
                <div class="arc-header" @click="toggleExpand(arc.id)">
                  <span class="expand-icon">{{ expanded[arc.id] ? '▼' : '▶' }}</span>
                  <span class="arc-title">{{ arc.title }}</span>
                  <StatusBadge :status="(arc.status || '').toLowerCase()" :label="arc.status" />
                  <span class="arc-meta">{{ arc.description }}</span>
                  <button class="action-btn" @click.stop="openEdit(arc)">编辑</button>
                  <button class="action-btn danger" @click.stop="handleDelete(arc)">删除</button>
                </div>
                <div class="arc-body" v-if="expanded[arc.id]">
                  <template v-for="ch in arc.children" :key="ch.id">
                    <div class="tree-chapter">
                      <div class="chapter-header" @click="toggleExpand(ch.id)">
                        <span class="expand-icon" v-if="ch.children?.length">{{ expanded[ch.id] ? '▼' : '▶' }}</span>
                        <span class="expand-icon" v-else>　</span>
                        <span class="chapter-title">{{ ch.title }}</span>
                        <StatusBadge :status="(ch.status || '').toLowerCase()" :label="ch.status" />
                        <span class="chapter-meta">{{ ch.description }}</span>
                        <button class="action-btn" @click.stop="openEdit(ch)">编辑</button>
                        <button class="action-btn danger" @click.stop="handleDelete(ch)">删除</button>
                        <button class="write-btn" @click.stop="goToWrite(ch)">写作</button>
                      </div>
                      <div class="chapter-body" v-if="expanded[ch.id] && ch.children?.length">
                        <div v-for="scene in ch.children" :key="scene.id" class="tree-scene" @click="goToWrite(scene)">
                          <span class="scene-dot" :class="(scene.status || '').toLowerCase()"></span>
                          <span class="scene-title">{{ scene.title }}</span>
                          <span class="scene-time">{{ scene.attributes?.time || '' }}</span>
                          <button class="action-btn" @click.stop="openEdit(scene)">编辑</button>
                          <button class="action-btn danger" @click.stop="handleDelete(scene)">删除</button>
                        </div>
                      </div>
                    </div>
                  </template>
                </div>
              </div>
            </template>
          </div>
        </div>
      </template>
    </div>

    <!-- Create Dialog -->
    <NeDialog v-model="showCreateDialog" title="新建卷/章节" size="md">
      <form @submit.prevent="handleCreate" class="entity-form">
        <div class="form-group">
          <label class="form-label">类型 *</label>
          <select v-model="form.node_type" class="form-select">
            <option value="Volume">卷 (Volume)</option>
            <option value="Arc">弧线 (Arc)</option>
            <option value="Chapter">章节 (Chapter)</option>
            <option value="Scene">场景 (Scene)</option>
          </select>
        </div>
        <div class="form-group">
          <label class="form-label">标题 *</label>
          <input v-model="form.title" class="form-input" placeholder="请输入标题" required />
        </div>
        <div class="form-group">
          <label class="form-label">描述</label>
          <textarea v-model="form.description" class="form-textarea" placeholder="请输入描述" rows="3"></textarea>
        </div>
        <div class="form-group" v-if="form.parent_id !== null">
          <label class="form-label">父节点 ID (可选)</label>
          <input v-model="form.parent_id" class="form-input" placeholder="留空则为顶级节点" />
        </div>
      </form>
      <template #footer>
        <button class="btn-secondary" @click="showCreateDialog = false">取消</button>
        <button class="btn-primary" @click="handleCreate">创建</button>
      </template>
    </NeDialog>

    <!-- Edit Dialog -->
    <NeDialog v-model="showEditDialog" title="编辑节点" size="md">
      <form @submit.prevent="handleEditSave" class="entity-form">
        <div class="form-group">
          <label class="form-label">标题 *</label>
          <input v-model="editForm.title" class="form-input" placeholder="请输入标题" required />
        </div>
        <div class="form-group">
          <label class="form-label">描述</label>
          <textarea v-model="editForm.description" class="form-textarea" placeholder="请输入描述" rows="3"></textarea>
        </div>
        <div class="form-group">
          <label class="form-label">内容</label>
          <textarea v-model="editForm.content" class="form-textarea" placeholder="请输入内容" rows="4"></textarea>
        </div>
        <div class="form-group">
          <label class="form-label">状态</label>
          <select v-model="editForm.status" class="form-select">
            <option v-for="s in statusOptions" :key="s.value" :value="s.value">{{ s.label }}</option>
          </select>
        </div>
      </form>
      <template #footer>
        <button class="btn-secondary" @click="showEditDialog = false">取消</button>
        <button class="btn-primary" @click="handleEditSave">保存</button>
      </template>
    </NeDialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useStoryStore } from '@/stores/story'
import type { NarrativeNode, NarrativeNodeStatus, TreeNode } from '@/types'
import StatusBadge from '@/components/ui/StatusBadge.vue'
import NeDialog from '@/components/ui/NeDialog.vue'

const route = useRoute()
const router = useRouter()
const storyStore = useStoryStore()
const projectId = route.params.id as string

const statusOptions: { value: NarrativeNodeStatus; label: string }[] = [
  { value: 'Draft', label: '草稿' },
  { value: 'Planned', label: '已规划' },
  { value: 'InProgress', label: '进行中' },
  { value: 'Completed', label: '已完成' },
  { value: 'Archived', label: '已归档' },
]

const expanded = ref<Record<string, boolean>>({ 'vol-1': true, 'arc-2': true, 'ch-4': true })
const showCreateDialog = ref(false)
const showEditDialog = ref(false)
const editingNode = ref<NarrativeNode | null>(null)
const form = ref({ node_type: 'Volume', title: '', description: '', parent_id: null as string | null })
const editForm = ref({ title: '', description: '', content: '', status: 'Draft' as NarrativeNodeStatus })

async function handleCreate() {
  if (!form.value.title.trim()) return
  await storyStore.createNode(projectId, {
    node_type: form.value.node_type as any,
    title: form.value.title.trim(),
    description: form.value.description.trim() || undefined,
    parent_id: form.value.parent_id || undefined,
  } as any)
  showCreateDialog.value = false
  form.value = { node_type: 'Volume', title: '', description: '', parent_id: null }
}

function openEdit(node: TreeNode) {
  editingNode.value = node as unknown as NarrativeNode
  editForm.value = {
    title: node.title,
    description: node.description || '',
    content: (node as any).content || '',
    status: node.status as NarrativeNodeStatus,
  }
  showEditDialog.value = true
}

async function handleEditSave() {
  if (!editingNode.value) return
  if (!editForm.value.title.trim()) return
  await storyStore.updateNode(editingNode.value.id, {
    title: editForm.value.title.trim(),
    description: editForm.value.description.trim() || undefined,
    content: editForm.value.content.trim() || undefined,
    status: editForm.value.status,
  })
  showEditDialog.value = false
  editingNode.value = null
}

async function handleDelete(node: TreeNode) {
  if (!confirm(`确认删除「${node.title}」？此操作不可撤销，其子节点也会被删除。`)) return
  await storyStore.deleteNode(node.id)
}

function toggleExpand(id: string) {
  expanded.value[id] = !expanded.value[id]
}

function goToWrite(node: any) {
  if (node.node_type === 'Scene') {
    router.push('/project/' + route.params.id + '/write/' + node.id)
  } else if (node.children?.length) {
    const firstScene = findFirstScene(node)
    if (firstScene) router.push('/project/' + route.params.id + '/write/' + firstScene.id)
  }
}

function findFirstScene(node: any): any {
  if (node.node_type === 'Scene') return node
  for (const child of node.children || []) {
    const found = findFirstScene(child)
    if (found) return found
  }
  return null
}

onMounted(async () => {
  await storyStore.fetchNodes(projectId)
})
</script>

<style scoped>
.story-page { height: 100%; overflow-y: auto; padding: var(--space-6) var(--space-8); }
.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--space-6); }
.page-title { font-size: var(--text-2xl); font-weight: 700; font-family: var(--font-serif); }
.btn-primary { padding: var(--space-2) var(--space-4); background: var(--color-primary); border: none; color: white; border-radius: var(--radius-sm); font-size: var(--text-sm); cursor: pointer; }
.btn-primary:hover { background: var(--color-primary-hover); }

.error-banner { padding: var(--space-3) var(--space-4); background: var(--color-error-subtle); color: var(--color-error); border-radius: var(--radius-sm); margin-bottom: var(--space-4); font-size: var(--text-sm); }
.loading-state { display: flex; align-items: center; justify-content: center; padding: var(--space-16); color: var(--text-tertiary); }
.loading-text { font-size: var(--text-sm); }
.empty-state { display: flex; flex-direction: column; align-items: center; justify-content: center; padding: var(--space-16); color: var(--text-tertiary); }
.empty-icon { font-size: 48px; margin-bottom: var(--space-4); }
.empty-text { font-size: var(--text-sm); }

.story-tree { display: flex; flex-direction: column; gap: var(--space-4); }

.tree-volume { border: 1px solid var(--border-default); border-radius: var(--radius-md); background: var(--bg-panel); overflow: hidden; }
.volume-header { display: flex; align-items: center; gap: var(--space-3); padding: var(--space-4) var(--space-5); cursor: pointer; transition: background var(--transition-fast); }
.volume-header:hover { background: var(--bg-hover); }
.volume-title { font-size: var(--text-lg); font-weight: 600; }
.volume-meta { font-size: var(--text-sm); color: var(--text-secondary); margin-left: auto; }

.expand-icon { font-size: var(--text-xs); color: var(--text-tertiary); width: 16px; }

.volume-body { border-top: 1px solid var(--border-muted); }

.tree-arc { border-bottom: 1px solid var(--border-muted); }
.tree-arc:last-child { border-bottom: none; }
.arc-header { display: flex; align-items: center; gap: var(--space-3); padding: var(--space-3) var(--space-5) var(--space-3) var(--space-8); cursor: pointer; transition: background var(--transition-fast); }
.arc-header:hover { background: var(--bg-hover); }
.arc-title { font-size: var(--text-md); font-weight: 500; }
.arc-meta { font-size: var(--text-sm); color: var(--text-secondary); margin-left: auto; }

.arc-body { }

.tree-chapter { border-bottom: 1px solid var(--border-muted); }
.tree-chapter:last-child { border-bottom: none; }
.chapter-header { display: flex; align-items: center; gap: var(--space-3); padding: var(--space-3) var(--space-5) var(--space-3) calc(var(--space-8) + var(--space-4)); cursor: pointer; transition: background var(--transition-fast); }
.chapter-header:hover { background: var(--bg-hover); }
.chapter-title { font-size: var(--text-sm); font-weight: 500; }
.chapter-meta { font-size: var(--text-xs); color: var(--text-tertiary); margin-left: auto; }

.write-btn {
  padding: var(--space-1) var(--space-3);
  border: 1px solid var(--color-primary);
  background: transparent;
  color: var(--color-primary-text);
  border-radius: var(--radius-sm);
  font-size: var(--text-xs);
  cursor: pointer;
  transition: all var(--transition-fast);
}
.write-btn:hover { background: var(--color-primary-subtle); }

.action-btn {
  padding: var(--space-1) var(--space-3);
  border: 1px solid var(--border-default);
  background: transparent;
  color: var(--text-secondary);
  border-radius: var(--radius-sm);
  font-size: var(--text-xs);
  cursor: pointer;
  transition: all var(--transition-fast);
}
.action-btn:hover { border-color: var(--border-emphasis); color: var(--text-primary); }
.action-btn.danger { color: var(--color-error); border-color: var(--color-error-border); }
.action-btn.danger:hover { background: var(--color-error-subtle); }

.chapter-body { }

.tree-scene {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-2) var(--space-5) var(--space-2) calc(var(--space-8) + var(--space-8));
  cursor: pointer;
  transition: background var(--transition-fast);
  font-size: var(--text-sm);
}
.tree-scene:hover { background: var(--bg-hover); }
.scene-dot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; }
.scene-dot.completed { background: var(--color-success); }
.scene-dot.inprogress { background: var(--color-accent); }
.scene-dot.planned { background: var(--text-tertiary); }
.scene-dot.draft { background: var(--color-warning); }
.scene-title { color: var(--text-secondary); }
.scene-time { margin-left: auto; font-size: var(--text-xs); color: var(--text-tertiary); }

.entity-form { display: flex; flex-direction: column; gap: var(--space-4); }
.form-group { display: flex; flex-direction: column; gap: var(--space-1); }
.form-label { font-size: var(--text-sm); font-weight: 500; color: var(--text-secondary); }
.form-input, .form-select, .form-textarea { padding: var(--space-2) var(--space-3); background: var(--bg-base); border: 1px solid var(--border-default); border-radius: var(--radius-sm); color: var(--text-primary); font-size: var(--text-sm); outline: none; }
.form-input:focus, .form-select:focus, .form-textarea:focus { border-color: var(--color-primary); }
.form-textarea { resize: vertical; font-family: inherit; }
.btn-secondary { padding: var(--space-2) var(--space-4); background: transparent; border: 1px solid var(--border-default); color: var(--text-secondary); border-radius: var(--radius-sm); font-size: var(--text-sm); cursor: pointer; }
.btn-secondary:hover { border-color: var(--border-emphasis); color: var(--text-primary); }
</style>
