<template>
  <div class="foreshadows-page">
    <div class="page-header">
      <h1 class="page-title">伏笔</h1>
      <button class="btn-primary" @click="showCreateDialog = true">+ 新建伏笔</button>
    </div>
    <div v-if="storyStore.foreshadows.length" class="foreshadow-list">
      <div v-for="fs in storyStore.foreshadows" :key="fs.id" class="foreshadow-card">
        <div class="fs-header">
          <span class="fs-badge" :class="(fs.status || '').toLowerCase()">{{ fs.status }}</span>
          <span class="fs-name">{{ fs.name }}</span>
          <span class="fs-importance">{{ fs.importance }}</span>
          <span class="fs-hint">暗示级别：{{ fs.hint_level }}</span>
        </div>
        <div class="fs-desc">{{ fs.description }}</div>
      </div>
    </div>
    <div v-else class="empty-state">
      <p>暂无伏笔</p>
    </div>

    <!-- Create Dialog -->
    <NeDialog v-model="showCreateDialog" title="新建伏笔" size="md">
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
          <label class="form-label">重要性</label>
          <select v-model="form.importance" class="form-select">
            <option value="Core">核心</option>
            <option value="Important">重要</option>
            <option value="Normal">一般</option>
          </select>
        </div>
        <div class="form-group">
          <label class="form-label">暗示级别</label>
          <select v-model="form.hint_level" class="form-select">
            <option value="Subtle">隐晦</option>
            <option value="Direct">直接</option>
            <option value="Obvious">明显</option>
          </select>
        </div>
      </form>
      <template #footer>
        <button class="btn-secondary" @click="showCreateDialog = false">取消</button>
        <button class="btn-primary" @click="handleSubmit">创建</button>
      </template>
    </NeDialog>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useRoute } from 'vue-router'
import { useStoryStore } from '@/stores/story'
import NeDialog from '@/components/ui/NeDialog.vue'

const route = useRoute()
const storyStore = useStoryStore()

const showCreateDialog = ref(false)
const form = ref({ name: '', description: '', importance: 'Normal', hint_level: 'Subtle' })

async function handleSubmit() {
  if (!form.value.name.trim()) return
  const projectId = route.params.id as string
  await storyStore.createForeshadow(projectId, {
    name: form.value.name.trim(),
    description: form.value.description.trim() || undefined,
    importance: form.value.importance as any,
    hint_level: form.value.hint_level as any,
  })
  showCreateDialog.value = false
  form.value = { name: '', description: '', importance: 'Normal', hint_level: 'Subtle' }
}
</script>

<style scoped>
.foreshadows-page { height: 100%; overflow-y: auto; padding: var(--space-6) var(--space-8); }
.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--space-6); }
.page-title { font-size: var(--text-2xl); font-weight: 700; font-family: var(--font-serif); }
.btn-primary { padding: var(--space-2) var(--space-4); background: var(--color-primary); border: none; color: white; border-radius: var(--radius-sm); font-size: var(--text-sm); cursor: pointer; }
.btn-primary:hover { background: var(--color-primary-hover); }
.foreshadow-list { display: flex; flex-direction: column; gap: var(--space-3); }
.foreshadow-card { padding: var(--space-4) var(--space-5); border: 1px solid var(--border-default); border-radius: var(--radius-md); background: var(--bg-panel); }
.fs-header { display: flex; align-items: center; gap: var(--space-3); margin-bottom: var(--space-2); }
.fs-badge { font-size: 10px; padding: 2px 8px; border-radius: 3px; }
.fs-badge.introduced { background: var(--color-info-subtle); color: var(--color-info); }
.fs-badge.active { background: var(--color-warning-subtle); color: var(--color-warning); }
.fs-badge.planned { background: var(--bg-panel-secondary); color: var(--text-tertiary); }
.fs-name { font-size: var(--text-md); font-weight: 600; }
.fs-importance { font-size: var(--text-xs); color: var(--text-tertiary); }
.fs-hint { font-size: var(--text-xs); color: var(--text-tertiary); margin-left: auto; }
.fs-desc { font-size: var(--text-sm); color: var(--text-secondary); }
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
</style>
