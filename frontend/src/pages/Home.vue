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
          <button class="btn-primary" @click="showCreateDialog = true">创建新项目</button>
        </div>
      </div>
    </div>

    <!-- Recent Projects -->
    <div class="home-section">
      <h2 class="section-title">最近项目</h2>
      <div v-if="projectStore.projects.length" class="project-grid">
        <div
          v-for="project in projectStore.projects"
          :key="project.id"
          class="project-card"
          @click="$router.push('/project/' + project.id)"
        >
          <div class="project-card-header">
            <span class="project-status" :class="(project.status || '').toLowerCase()">{{ statusLabels[project.status] || project.status }}</span>
          </div>
          <h3 class="project-name">{{ project.name }}</h3>
          <p class="project-desc">{{ project.description || '暂无描述' }}</p>
          <div class="project-meta">
            <span>更新于 {{ formatDate(project.updated_at) }}</span>
          </div>
        </div>
      </div>
      <div v-else class="empty-state">
        <p>暂无项目，点击"创建新项目"开始</p>
      </div>
    </div>

    <!-- Architecture Overview -->
    <div class="home-section">
      <h2 class="section-title">系统架构</h2>
      <div class="arch-flow">
        <div class="arch-step">
          <span class="arch-icon">🌍</span>
          <span class="arch-label">World</span>
          <span class="arch-desc">世界事实</span>
        </div>
        <span class="arch-arrow">→</span>
        <div class="arch-step">
          <span class="arch-icon">📖</span>
          <span class="arch-label">Story</span>
          <span class="arch-desc">叙事结构</span>
        </div>
        <span class="arch-arrow">→</span>
        <div class="arch-step">
          <span class="arch-icon">🧠</span>
          <span class="arch-label">Context</span>
          <span class="arch-desc">AI 上下文</span>
        </div>
        <span class="arch-arrow">→</span>
        <div class="arch-step">
          <span class="arch-icon">🤖</span>
          <span class="arch-label">Generation</span>
          <span class="arch-desc">AI 生成</span>
        </div>
        <span class="arch-arrow">→</span>
        <div class="arch-step">
          <span class="arch-icon">📋</span>
          <span class="arch-label">Proposal</span>
          <span class="arch-desc">AI 提案</span>
        </div>
        <span class="arch-arrow">→</span>
        <div class="arch-step">
          <span class="arch-icon">✅</span>
          <span class="arch-label">Validation</span>
          <span class="arch-desc">系统审查</span>
        </div>
        <span class="arch-arrow">→</span>
        <div class="arch-step">
          <span class="arch-icon">📝</span>
          <span class="arch-label">Commit</span>
          <span class="arch-desc">提交变更</span>
        </div>
      </div>
    </div>

    <!-- Create Project Dialog -->
    <NeDialog v-model="showCreateDialog" title="创建新项目" size="md">
      <form @submit.prevent="handleCreate" class="entity-form">
        <div class="form-group">
          <label class="form-label">项目名称 *</label>
          <input v-model="newProject.name" class="form-input" placeholder="请输入项目名称" required />
        </div>
        <div class="form-group">
          <label class="form-label">项目描述</label>
          <textarea v-model="newProject.description" class="form-textarea" placeholder="请输入项目描述" rows="3"></textarea>
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
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useProjectStore } from '@/stores/project'
import NeDialog from '@/components/ui/NeDialog.vue'

const router = useRouter()
const projectStore = useProjectStore()

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

onMounted(() => {
  projectStore.fetchProjects()
})

function formatDate(dateStr: string) {
  try {
    const d = new Date(dateStr)
    return `${d.getMonth() + 1}月${d.getDate()}日`
  } catch {
    return dateStr
  }
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
.project-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: var(--space-4);
}
.project-card {
  padding: var(--space-5);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-lg);
  background: var(--bg-panel);
  cursor: pointer;
  transition: all var(--transition-fast);
}
.project-card:hover { border-color: var(--border-emphasis); transform: translateY(-2px); }
.project-card-header { margin-bottom: var(--space-3); }
.project-status {
  font-size: var(--text-xs);
  padding: 2px 8px;
  border-radius: 10px;
}
.project-status.concept { background: var(--bg-panel-secondary); color: var(--text-tertiary); }
.project-status.planning { background: var(--color-info-subtle); color: var(--color-info); }
.project-status.writing { background: var(--color-success-subtle); color: var(--color-success); }
.project-name {
  font-size: var(--text-lg);
  font-weight: 600;
  margin-bottom: var(--space-2);
}
.project-desc {
  font-size: var(--text-sm);
  color: var(--text-secondary);
  margin-bottom: var(--space-4);
  line-height: 1.6;
}
.project-meta {
  font-size: var(--text-xs);
  color: var(--text-tertiary);
}
.empty-state {
  padding: var(--space-12);
  text-align: center;
  color: var(--text-tertiary);
}
.arch-flow {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-2);
  flex-wrap: wrap;
  padding: var(--space-6);
  background: var(--bg-panel);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-lg);
}
.arch-step {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-1);
  padding: var(--space-3);
}
.arch-icon { font-size: 24px; }
.arch-label { font-size: var(--text-sm); font-weight: 600; }
.arch-desc { font-size: var(--text-xs); color: var(--text-tertiary); }
.arch-arrow { color: var(--text-tertiary); font-size: var(--text-lg); }

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
