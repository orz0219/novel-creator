<template>
  <div class="foreshadows-page">
    <div class="page-header">
      <h1 class="page-title">伏笔</h1>
      <button class="btn-primary" @click="openCreate">+ 新建伏笔</button>
    </div>

    <div v-if="storyStore.foreshadows.length" class="foreshadow-list">
      <div v-for="fs in storyStore.foreshadows" :key="fs.id" class="foreshadow-card">
        <div class="fs-header">
          <span class="fs-badge" :class="(fs.status || '').toLowerCase()">{{ statusLabel[fs.status] }}</span>
          <span class="fs-name">{{ fs.name }}</span>
          <span class="fs-importance" :class="'imp-' + (fs.importance || '').toLowerCase()">{{ importanceLabel[fs.importance] }}</span>
          <span class="fs-hint">暗示级别：{{ hintLabel[fs.hint_level] }}</span>
          <button class="btn-edit" @click="openEdit(fs)">编辑</button>
          <button class="btn-danger" @click="handleDelete(fs)">删除</button>
        </div>

        <div v-if="fs.description" class="fs-desc">{{ fs.description }}</div>

        <div class="fs-meta">
          <div v-if="fs.planted_scene_id" class="fs-meta-row">
            <span class="fs-meta-label">种植场景</span>
            <span class="fs-meta-value">{{ fs.planted_scene_id }}</span>
          </div>
          <div v-if="fs.revealed_scene_id" class="fs-meta-row">
            <span class="fs-meta-label">揭示场景</span>
            <span class="fs-meta-value">{{ fs.revealed_scene_id }}</span>
          </div>
          <div v-if="fs.related_entity_ids && fs.related_entity_ids.length" class="fs-meta-row">
            <span class="fs-meta-label">关联实体</span>
            <span class="fs-meta-value">关联 {{ fs.related_entity_ids.length }} 个实体</span>
          </div>
        </div>
      </div>
    </div>
    <div v-else class="empty-state">
      <p>暂无伏笔</p>
    </div>

    <!-- Create/Edit Dialog -->
    <NeDialog v-model="showDialog" :title="editing ? '编辑伏笔' : '新建伏笔'" size="md">
      <form @submit.prevent="handleSubmit" class="entity-form">
        <div class="form-group">
          <label class="form-label">名称 *</label>
          <input v-model="form.name" class="form-input" placeholder="伏笔名称" required />
        </div>
        <div class="form-group">
          <label class="form-label">描述</label>
          <textarea v-model="form.description" class="form-textarea" placeholder="伏笔描述" rows="3"></textarea>
        </div>
        <div class="form-group">
          <label class="form-label">状态</label>
          <select v-model="form.status" class="form-select">
            <option value="Planned">计划中</option>
            <option value="Introduced">已引入</option>
            <option value="Active">进行中</option>
            <option value="Revealed">已揭示</option>
            <option value="Abandoned">已放弃</option>
          </select>
        </div>
        <div class="form-group">
          <label class="form-label">重要性</label>
          <select v-model="form.importance" class="form-select">
            <option value="Core">核心</option>
            <option value="Important">重要</option>
            <option value="Normal">普通</option>
            <option value="Minor">次要</option>
          </select>
        </div>
        <div class="form-group">
          <label class="form-label">暗示级别</label>
          <select v-model="form.hint_level" class="form-select">
            <option value="Explicit">明示</option>
            <option value="Direct">直接</option>
            <option value="Subtle">隐晦</option>
            <option value="Hidden">隐藏</option>
          </select>
        </div>
        <div class="form-group">
          <label class="form-label">种植场景 ID</label>
          <input v-model="form.planted_scene_id" class="form-input" placeholder="种植场景 ID（可选）" />
        </div>
        <div class="form-group">
          <label class="form-label">揭示场景 ID</label>
          <input v-model="form.revealed_scene_id" class="form-input" placeholder="揭示场景 ID（可选）" />
        </div>
        <div class="form-group">
          <label class="form-label">关联实体 ID（逗号分隔）</label>
          <input v-model="relatedInput" class="form-input" placeholder="实体 ID1, 实体 ID2（可选）" />
        </div>
      </form>
      <template #footer>
        <button class="btn-secondary" @click="showDialog = false">取消</button>
        <button class="btn-primary" @click="handleSubmit">{{ editing ? '保存' : '创建' }}</button>
      </template>
    </NeDialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { useStoryStore } from '@/stores/story'
import type { Foreshadowing, ForeshadowingStatus, ForeshadowingImportance, HintLevel } from '@/types/narrative'
import NeDialog from '@/components/ui/NeDialog.vue'

const route = useRoute()
const storyStore = useStoryStore()
const projectId = route.params.id as string

const statusLabel: Record<ForeshadowingStatus, string> = {
  Planned: '计划中',
  Introduced: '已引入',
  Active: '进行中',
  Revealed: '已揭示',
  Abandoned: '已放弃',
}

const importanceLabel: Record<ForeshadowingImportance, string> = {
  Core: '核心',
  Important: '重要',
  Normal: '普通',
  Minor: '次要',
}

const hintLabel: Record<HintLevel, string> = {
  Explicit: '明示',
  Direct: '直接',
  Subtle: '隐晦',
  Hidden: '隐藏',
}

const showDialog = ref(false)
const editing = ref(false)
const editingId = ref<string | null>(null)
const form = ref<{
  name: string
  description: string
  status: ForeshadowingStatus
  importance: ForeshadowingImportance
  hint_level: HintLevel
  planted_scene_id: string
  revealed_scene_id: string
}>({
  name: '',
  description: '',
  status: 'Planned',
  importance: 'Normal',
  hint_level: 'Subtle',
  planted_scene_id: '',
  revealed_scene_id: '',
})
const relatedInput = ref('')

const relatedEntityIds = computed(() =>
  relatedInput.value
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean),
)

function resetForm() {
  form.value = {
    name: '',
    description: '',
    status: 'Planned',
    importance: 'Normal',
    hint_level: 'Subtle',
    planted_scene_id: '',
    revealed_scene_id: '',
  }
  relatedInput.value = ''
}

function openCreate() {
  editing.value = false
  editingId.value = null
  resetForm()
  showDialog.value = true
}

function openEdit(fs: Foreshadowing) {
  editing.value = true
  editingId.value = fs.id
  form.value = {
    name: fs.name,
    description: fs.description ?? '',
    status: fs.status,
    importance: fs.importance,
    hint_level: fs.hint_level,
    planted_scene_id: fs.planted_scene_id ?? '',
    revealed_scene_id: fs.revealed_scene_id ?? '',
  }
  relatedInput.value = (fs.related_entity_ids ?? []).join(', ')
  showDialog.value = true
}

async function handleDelete(fs: Foreshadowing) {
  if (!confirm(`确认删除伏笔「${fs.name}」？此操作不可撤销。`)) return
  await storyStore.deleteForeshadow(fs.id)
}

async function handleSubmit() {
  if (!form.value.name.trim()) return
  const payload = {
    name: form.value.name.trim(),
    description: form.value.description.trim() || undefined,
    status: form.value.status,
    importance: form.value.importance,
    hint_level: form.value.hint_level,
    planted_scene_id: form.value.planted_scene_id.trim() || undefined,
    revealed_scene_id: form.value.revealed_scene_id.trim() || undefined,
    related_entity_ids: relatedEntityIds.value,
  }

  if (editing.value && editingId.value) {
    await storyStore.updateForeshadow(editingId.value, payload)
  } else {
    await storyStore.createForeshadow(projectId, payload)
  }
  showDialog.value = false
  editing.value = false
  editingId.value = null
  resetForm()
}

onMounted(async () => {
  await storyStore.fetchForeshadows(projectId)
})
</script>

<style scoped>
.foreshadows-page { height: 100%; overflow-y: auto; padding: var(--space-6) var(--space-8); }
.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--space-6); }
.page-title { font-size: var(--text-2xl); font-weight: 700; font-family: var(--font-serif); }
.btn-primary { padding: var(--space-2) var(--space-4); background: var(--color-primary); border: none; color: white; border-radius: var(--radius-sm); font-size: var(--text-sm); cursor: pointer; }
.btn-primary:hover { background: var(--color-primary-hover); }
.btn-secondary {
  padding: var(--space-2) var(--space-4);
  background: transparent;
  border: 1px solid var(--border-default);
  color: var(--text-secondary);
  border-radius: var(--radius-sm);
  font-size: var(--text-sm);
  cursor: pointer;
}
.btn-secondary:hover { border-color: var(--border-emphasis); color: var(--text-primary); }
.btn-edit {
  margin-left: auto;
  padding: var(--space-1) var(--space-3);
  background: transparent;
  border: 1px solid var(--border-default);
  color: var(--text-secondary);
  border-radius: var(--radius-sm);
  font-size: var(--text-xs);
  cursor: pointer;
}
.btn-edit:hover { border-color: var(--border-emphasis); color: var(--text-primary); }
.btn-danger {
  padding: var(--space-1) var(--space-3);
  background: transparent;
  border: 1px solid var(--border-default);
  color: var(--color-error);
  border-radius: var(--radius-sm);
  font-size: var(--text-xs);
  cursor: pointer;
}
.btn-danger:hover { border-color: var(--color-error); background: var(--color-error-subtle); }
.foreshadow-list { display: flex; flex-direction: column; gap: var(--space-3); }
.foreshadow-card { padding: var(--space-4) var(--space-5); border: 1px solid var(--border-default); border-radius: var(--radius-md); background: var(--bg-panel); }
.fs-header { display: flex; align-items: center; gap: var(--space-3); margin-bottom: var(--space-2); }
.fs-badge { font-size: 10px; padding: 2px 8px; border-radius: 3px; }
.fs-badge.planned { background: var(--bg-panel-secondary); color: var(--text-tertiary); }
.fs-badge.introduced { background: var(--color-info-subtle); color: var(--color-info); }
.fs-badge.active { background: var(--color-warning-subtle); color: var(--color-warning); }
.fs-badge.revealed { background: var(--color-success-subtle); color: var(--color-success); }
.fs-badge.abandoned { background: var(--color-error-subtle); color: var(--color-error); }
.fs-name { font-size: var(--text-md); font-weight: 600; }
.fs-importance { font-size: var(--text-xs); padding: 1px 6px; border-radius: 3px; }
.fs-importance.imp-core { background: var(--color-error-subtle); color: var(--color-error); }
.fs-importance.imp-important { background: var(--color-warning-subtle); color: var(--color-warning); }
.fs-importance.imp-normal { background: var(--bg-panel-secondary); color: var(--text-secondary); }
.fs-importance.imp-minor { background: var(--bg-panel-secondary); color: var(--text-tertiary); }
.fs-hint { font-size: var(--text-xs); color: var(--text-tertiary); }
.fs-desc { font-size: var(--text-sm); color: var(--text-secondary); margin-bottom: var(--space-2); }
.fs-meta { display: flex; flex-direction: column; gap: var(--space-1); }
.fs-meta-row { display: flex; gap: var(--space-3); font-size: var(--text-xs); }
.fs-meta-label { flex: 0 0 64px; color: var(--text-tertiary); }
.fs-meta-value { color: var(--text-secondary); }
.empty-state { padding: var(--space-12); text-align: center; color: var(--text-tertiary); }
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
.form-select {
  padding: var(--space-2) var(--space-3);
  background: var(--bg-base);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: var(--text-sm);
  outline: none;
}
</style>
