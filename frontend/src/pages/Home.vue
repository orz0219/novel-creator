<template>
  <div class="home-page">
    <!-- Hero Section -->
    <div class="home-hero">
      <div class="hero-content">
        <div class="hero-badge">Novel Engine</div>
        <h1 class="hero-title">小说创作工坊</h1>
        <p class="hero-desc">
          一个结构化、可验证、可追踪的小说世界运行引擎。<br/>
          AI 在你构建的世界中帮助你创作。
        </p>
        <div class="hero-actions">
          <button class="btn-primary" @click="openCreate">创建新项目</button>
        </div>
      </div>
    </div>

    <!-- Recent Projects -->
    <div class="home-section">


      <div v-if="projectStore.error" class="error-banner">{{ projectStore.error }}</div>

      <div v-else-if="projectStore.loading" class="loading-state">
        <Loader2 class="loading-icon" :size="24" />
        <span class="loading-text">加载中...</span>
      </div>

      <template v-else>
        <div v-if="projectStore.projects.length" class="list-toolbar">
          <div class="search-box">
            <Search class="search-icon" :size="16" />
            <NeInput v-model="searchQuery" placeholder="搜索项目名称或描述" />
          </div>
        </div>

        <div v-if="filteredProjects.length" class="project-table-wrap">
          <table class="project-table">
            <thead>
              <tr>
                <th>名称</th>
                <th>描述</th>
                <th>状态</th>
                <th>更新时间</th>
                <th class="col-actions">操作</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="project in filteredProjects" :key="project.id">
                <td class="cell-name">
                  <button class="cell-link" @click="$router.push('/project/' + project.id)">{{ project.name }}</button>
                </td>
                <td class="cell-desc" :title="project.description || '—'">{{ project.description || '—' }}</td>
                <td>
                  <span class="project-status" :class="(project.status || '').toLowerCase()">{{ statusLabels[project.status] || project.status }}</span>
                </td>
                <td class="cell-time">{{ formatDate(project.updated_at) }}</td>
                <td class="col-actions">
                  <div class="row-actions" @click.stop>
                    <button class="btn-ghost" @click="openEdit(project)">编辑</button>
                    <button class="btn-danger" @click="openDelete(project)">删除</button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <div v-else-if="projectStore.projects.length" class="empty-state">
          <Search class="empty-icon" :size="40" />
          <span class="empty-text">未找到匹配的项目</span>
        </div>

        <div v-else class="empty-state">
          <BookOpen class="empty-icon" :size="40" />
          <span class="empty-text">暂无项目，点击"创建新项目"开始</span>
        </div>
      </template>
    </div>

    <!-- Create Project Dialog -->
    <NeDialog v-model="showCreateDialog" title="创建新项目" size="md">
      <form @submit.prevent="handleCreate" class="entity-form">
        <div class="form-group">
          <label class="form-label">项目名称 *（最多 10 字）</label>
          <input v-model="newProject.name" class="form-input" maxlength="10" placeholder="请输入项目名称" required />
        </div>
        <div class="form-group">
          <label class="form-label">项目描述（最多 500 字）</label>
          <textarea v-model="newProject.description" class="form-textarea" maxlength="500" placeholder="请输入项目描述" rows="3"></textarea>
        </div>
        <div v-if="createError" class="form-error">{{ createError }}</div>
      </form>
      <template #footer>
        <button class="btn-secondary" @click="showCreateDialog = false">取消</button>
        <button class="btn-primary" :disabled="creating" @click="handleCreate">
          {{ creating ? '创建中...' : '创建' }}
        </button>
      </template>
    </NeDialog>

    <!-- Edit Project Dialog -->
    <NeDialog v-model="showEditDialog" title="编辑项目" size="md">
      <form @submit.prevent="handleEdit" class="entity-form">
        <div class="form-group">
          <label class="form-label">项目名称 *（最多 10 字）</label>
          <input v-model="editForm.name" class="form-input" maxlength="10" placeholder="请输入项目名称" required />
        </div>
        <div class="form-group">
          <label class="form-label">项目描述（最多 500 字）</label>
          <textarea v-model="editForm.description" class="form-textarea" maxlength="500" placeholder="请输入项目描述" rows="3"></textarea>
        </div>
        <div v-if="editError" class="form-error">{{ editError }}</div>
      </form>
      <template #footer>
        <button class="btn-secondary" @click="showEditDialog = false">取消</button>
        <button class="btn-primary" :disabled="saving" @click="handleEdit">
          {{ saving ? '保存中...' : '保存' }}
        </button>
      </template>
    </NeDialog>

    <!-- Delete Confirm Dialog -->
    <NeDialog v-model="showDeleteDialog" title="删除项目" size="sm">
      <p class="confirm-text">
        确认删除项目「<strong>{{ deleteTarget?.name }}</strong>」？此操作不可撤销。
      </p>
      <template #footer>
        <button class="btn-secondary" @click="showDeleteDialog = false">取消</button>
        <button class="btn-danger-solid" :disabled="deleting" @click="confirmDelete">
          {{ deleting ? '删除中...' : '删除' }}
        </button>
      </template>
    </NeDialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useProjectStore } from '@/stores/project'
import { useUiStore } from '@/stores/ui'
import type { Project } from '@/types/project'
import NeDialog from '@/components/ui/NeDialog.vue'
import NeInput from '@/components/ui/NeInput.vue'
import { BookOpen, Loader2, Search } from 'lucide-vue-next'

const router = useRouter()
const projectStore = useProjectStore()
const uiStore = useUiStore()

const searchQuery = ref('')
const filteredProjects = computed(() => {
  const q = searchQuery.value.trim().toLowerCase()
  if (!q) return projectStore.projects
  return projectStore.projects.filter(
    (p) => (p.name?.toLowerCase().includes(q)) || (p.description?.toLowerCase().includes(q)),
  )
})

const statusLabels: Record<string, string> = {
  Concept: '概念',
  Planning: '规划中',
  Writing: '创作中',
  Paused: '暂停',
  Completed: '已完成',
  Archived: '已归档',
}

const showCreateDialog = ref(false)
const creating = ref(false)
const createError = ref('')
const newProject = ref({ name: '', description: '' })

const showEditDialog = ref(false)
const saving = ref(false)
const editError = ref('')
const editingProject = ref<Project | null>(null)
const editForm = ref<{ name: string; description: string }>({
  name: '',
  description: '',
})

onMounted(async () => {
  await projectStore.fetchProjects()
})

function formatDate(dateStr: string) {
  try {
    const d = new Date(dateStr)
    return `${d.getFullYear()}年${d.getMonth() + 1}月${d.getDate()}日`
  } catch {
    return dateStr
  }
}

function openCreate() {
  newProject.value = { name: '', description: '' }
  createError.value = ''
  showCreateDialog.value = true
}

async function handleCreate() {
  if (!newProject.value.name.trim()) {
    createError.value = '请输入项目名称'
    return
  }
  creating.value = true
  createError.value = ''
  try {
    const project = await projectStore.createProject({
      name: newProject.value.name.trim(),
      description: newProject.value.description.trim() || undefined,
    })
    showCreateDialog.value = false
    newProject.value = { name: '', description: '' }
    router.push('/project/' + project.id)
  } catch (e: any) {
    createError.value = e.message || '创建失败'
  } finally {
    creating.value = false
  }
}

function openEdit(project: Project) {
  editingProject.value = project
  editForm.value = {
    name: project.name,
    description: project.description ?? '',
  }
  editError.value = ''
  showEditDialog.value = true
}

async function handleEdit() {
  if (!editingProject.value) return
  if (!editForm.value.name.trim()) {
    editError.value = '请输入项目名称'
    return
  }
  saving.value = true
  editError.value = ''
  try {
    await projectStore.updateProject(editingProject.value.id, {
      name: editForm.value.name.trim(),
      description: editForm.value.description.trim() || undefined,
    })
    showEditDialog.value = false
    editingProject.value = null
  } catch (e: any) {
    editError.value = e.message || '保存失败'
  } finally {
    saving.value = false
  }
}

const showDeleteDialog = ref(false)
const deleting = ref(false)
const deleteTarget = ref<Project | null>(null)

function openDelete(project: Project) {
  deleteTarget.value = project
  showDeleteDialog.value = true
}

async function confirmDelete() {
  if (!deleteTarget.value) return
  deleting.value = true
  const name = deleteTarget.value.name
  try {
    await projectStore.deleteProject(deleteTarget.value.id)
    uiStore.addToast({ type: 'success', title: '已删除项目', message: name })
    showDeleteDialog.value = false
    deleteTarget.value = null
  } catch (e: any) {
    uiStore.addToast({ type: 'error', title: '删除失败', message: e.message || '' })
    showDeleteDialog.value = false
    deleteTarget.value = null
  } finally {
    deleting.value = false
  }
}
</script>

<style scoped>
.home-page {
  height: 100%;
  overflow-y: auto;
  padding: var(--space-8) var(--space-16);
}
.home-hero {
  text-align: center;
  padding: var(--space-16) 0;
  margin-bottom: var(--space-8);
}
.hero-badge {
  display: inline-block;
  font-size: var(--text-xs);
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.1em;
  color: var(--color-primary-text);
  background: var(--color-primary-subtle);
  padding: var(--space-1) var(--space-4);
  border-radius: 20px;
  margin-bottom: var(--space-4);
}
.hero-title {
  font-size: 48px;
  font-weight: 700;
  font-family: var(--font-serif);
  margin-bottom: var(--space-4);
  background: linear-gradient(135deg, var(--text-primary) 0%, var(--color-primary-text) 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}
.hero-desc {
  font-size: var(--text-lg);
  color: var(--text-secondary);
  line-height: 1.8;
  max-width: 480px;
  margin: 0 auto var(--space-8);
}
.hero-actions {
  display: flex;
  gap: var(--space-3);
  justify-content: center;
}
.btn-primary {
  padding: var(--space-3) var(--space-6);
  background: var(--color-primary);
  border: none;
  color: white;
  border-radius: var(--radius-md);
  font-size: var(--text-md);
  font-weight: 500;
  cursor: pointer;
  transition: all var(--transition-fast);
}
.btn-primary:hover { background: var(--color-primary-hover); }
.btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }
.btn-secondary {
  padding: var(--space-3) var(--space-6);
  background: transparent;
  border: 1px solid var(--border-default);
  color: var(--text-secondary);
  border-radius: var(--radius-md);
  font-size: var(--text-md);
  cursor: pointer;
  transition: all var(--transition-fast);
}
.btn-secondary:hover { border-color: var(--border-emphasis); color: var(--text-primary); }
.home-section { margin-bottom: var(--space-12); }
.section-title {
  font-size: var(--text-xl);
  font-weight: 600;
  margin-bottom: var(--space-6);
}
.list-toolbar {
  display: flex;
  justify-content: flex-start;
  max-width: 880px;
  margin: 0 auto var(--space-4);
}
.search-box {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  width: 320px;
  max-width: 100%;
}
.search-icon { flex: 0 0 auto; color: var(--text-tertiary); }
.search-box :deep(.ne-input-wrapper) { flex: 1; min-width: 0; }
.search-box :deep(.ne-input) { width: 100%; }

/* 项目表格 */
.project-table-wrap {
  max-width: 880px;
  margin: 0 auto;
  border: 1px solid var(--border-default);
  border-radius: var(--radius-lg);
  overflow: hidden;
  background: var(--bg-panel);
}
.project-table {
  width: 100%;
  border-collapse: collapse;
  font-size: var(--text-sm);
}
.project-table th {
  text-align: left;
  padding: var(--space-3) var(--space-4);
  font-size: var(--text-xs);
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--text-tertiary);
  background: var(--bg-panel-secondary);
  border-bottom: 1px solid var(--border-default);
  white-space: nowrap;
}
.project-table td {
  padding: var(--space-3) var(--space-4);
  border-bottom: 1px solid var(--border-muted);
  color: var(--text-secondary);
  vertical-align: middle;
}
.project-table tbody tr:last-child td { border-bottom: none; }
.project-table tbody tr:hover { background: var(--bg-hover); }
.cell-name { font-weight: 600; color: var(--text-primary); }
.cell-link {
  background: none; border: none; padding: 0; cursor: pointer;
  font: inherit; font-weight: 600; color: var(--color-primary-text);
}
.cell-link:hover { text-decoration: underline; }
.cell-desc {
  max-width: 360px;
  color: var(--text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.cell-time { white-space: nowrap; color: var(--text-tertiary); }
.col-actions { text-align: right; white-space: nowrap; }
.row-actions { display: inline-flex; align-items: center; gap: var(--space-2); }
.project-status {
  display: inline-block;
  font-size: var(--text-xs);
  padding: 2px 8px;
  border-radius: 10px;
  background: var(--bg-panel-secondary);
  color: var(--text-tertiary);
}
.project-status.concept { background: var(--bg-panel-secondary); color: var(--text-tertiary); }
.project-status.planning { background: var(--color-info-subtle); color: var(--color-info); }
.project-status.writing { background: var(--color-success-subtle); color: var(--color-success); }

.btn-ghost {
  padding: var(--space-2) var(--space-4);
  background: transparent;
  border: 1px solid var(--border-default);
  color: var(--text-secondary);
  border-radius: var(--radius-sm);
  font-size: var(--text-sm);
  cursor: pointer;
  transition: all var(--transition-fast);
}
.btn-ghost:hover { border-color: var(--border-emphasis); color: var(--text-primary); }
.btn-danger {
  padding: var(--space-2) var(--space-4);
  background: transparent;
  border: 1px solid var(--color-error);
  color: var(--color-error);
  border-radius: var(--radius-sm);
  font-size: var(--text-sm);
  cursor: pointer;
  transition: all var(--transition-fast);
}
.btn-danger:hover { background: var(--color-error-subtle); }
.btn-danger-solid {
  padding: var(--space-3) var(--space-6);
  background: var(--color-error);
  border: none;
  color: white;
  border-radius: var(--radius-md);
  font-size: var(--text-md);
  font-weight: 500;
  cursor: pointer;
  transition: filter var(--transition-fast);
}
.btn-danger-solid:hover { filter: brightness(0.92); }
.btn-danger-solid:disabled { opacity: 0.5; cursor: not-allowed; }
.confirm-text {
  font-size: var(--text-sm);
  color: var(--text-secondary);
  line-height: 1.7;
  margin: 0;
}
.confirm-text strong { color: var(--text-primary); font-weight: 600; }
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: var(--space-12);
  color: var(--text-tertiary);
}
.empty-icon { font-size: 48px; margin-bottom: var(--space-4); }
.empty-text { font-size: var(--text-sm); }
.loading-state {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-12);
  color: var(--text-tertiary);
}
.loading-icon { font-size: 24px; }
.loading-text { font-size: var(--text-sm); }
.error-banner {
  padding: var(--space-3) var(--space-4);
  background: var(--color-error-subtle);
  color: var(--color-error);
  border-radius: var(--radius-sm);
  margin-bottom: var(--space-4);
  font-size: var(--text-sm);
}
/* Form styles */
.entity-form { display: flex; flex-direction: column; gap: var(--space-4); }
.form-group { display: flex; flex-direction: column; gap: var(--space-1); }
.form-label { font-size: var(--text-sm); font-weight: 500; color: var(--text-secondary); }
.form-input {
  padding: var(--space-2) var(--space-3);
  background: var(--bg-base);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: var(--text-sm);
  outline: none;
}
.form-input:focus { border-color: var(--color-primary); }
.form-textarea {
  padding: var(--space-2) var(--space-3);
  background: var(--bg-base);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: var(--text-sm);
  outline: none;
  resize: vertical;
  font-family: inherit;
}
.form-textarea:focus { border-color: var(--color-primary); }
.form-error { color: var(--color-error); font-size: var(--text-xs); padding: var(--space-2); background: var(--color-error-subtle); border-radius: var(--radius-sm); }
</style>
