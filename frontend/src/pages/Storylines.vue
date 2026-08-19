<template>
  <div class="storylines-page">
    <div class="page-header">
      <h1 class="page-title">剧情线</h1>
      <button class="btn-primary" @click="showCreateDialog = true">+ 新建剧情线</button>
    </div>
    <div v-if="storyStore.storylines.length" class="storyline-list">
      <div v-for="sl in storyStore.storylines" :key="sl.id" class="storyline-card">
        <div class="sl-header">
          <span class="sl-dot" :class="(sl.status || '').toLowerCase()"></span>
          <span class="sl-name">{{ sl.name }}</span>
          <span class="sl-importance" :class="(sl.importance || '').toLowerCase()">{{ sl.importance }}</span>
          <StatusBadge :status="(sl.status || '').toLowerCase()" :label="sl.status || ''" />
        </div>
        <div class="sl-desc">{{ sl.description }}</div>
      </div>
    </div>
    <div v-else class="empty-state">
      <p>暂无剧情线</p>
    </div>

    <!-- Create Dialog -->
    <NeDialog v-model="showCreateDialog" title="新建剧情线" size="md">
      <form @submit.prevent="handleSubmit" class="entity-form">
        <div class="form-group">
          <label class="form-label">名称 *</label>
          <input v-model="form.name" class="form-input" placeholder="剧情线名称" required />
        </div>
        <div class="form-group">
          <label class="form-label">描述</label>
          <textarea v-model="form.description" class="form-textarea" placeholder="剧情线描述" rows="3"></textarea>
        </div>
        <div class="form-group">
          <label class="form-label">重要性</label>
          <select v-model="form.importance" class="form-select">
            <option value="Main">主线</option>
            <option value="Important">重要</option>
            <option value="Normal">一般</option>
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
import StatusBadge from '@/components/ui/StatusBadge.vue'
import NeDialog from '@/components/ui/NeDialog.vue'

const route = useRoute()
const storyStore = useStoryStore()

const showCreateDialog = ref(false)
const form = ref({ name: '', description: '', importance: 'Normal' })

async function handleSubmit() {
  if (!form.value.name.trim()) return
  const projectId = route.params.id as string
  await storyStore.createStoryline(projectId, {
    name: form.value.name.trim(),
    description: form.value.description.trim() || undefined,
    importance: form.value.importance as any,
  })
  showCreateDialog.value = false
  form.value = { name: '', description: '', importance: 'Normal' }
}
</script>

<style scoped>
.storylines-page { height: 100%; overflow-y: auto; padding: var(--space-6) var(--space-8); }
.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--space-6); }
.page-title { font-size: var(--text-2xl); font-weight: 700; font-family: var(--font-serif); }
.btn-primary { padding: var(--space-2) var(--space-4); background: var(--color-primary); border: none; color: white; border-radius: var(--radius-sm); font-size: var(--text-sm); cursor: pointer; }
.btn-primary:hover { background: var(--color-primary-hover); }
.storyline-list { display: flex; flex-direction: column; gap: var(--space-3); }
.storyline-card { padding: var(--space-4) var(--space-5); border: 1px solid var(--border-default); border-radius: var(--radius-md); background: var(--bg-panel); }
.sl-header { display: flex; align-items: center; gap: var(--space-3); margin-bottom: var(--space-2); }
.sl-dot { width: 8px; height: 8px; border-radius: 50%; }
.sl-dot.active { background: var(--color-success); }
.sl-dot.planned { background: var(--text-tertiary); }
.sl-name { font-size: var(--text-md); font-weight: 600; }
.sl-importance { font-size: var(--text-xs); padding: 2px 8px; border-radius: 10px; }
.sl-importance.main { background: var(--color-primary-subtle); color: var(--color-primary-text); }
.sl-importance.important { background: var(--color-accent-subtle); color: var(--color-accent); }
.sl-importance.normal { background: var(--bg-panel-secondary); color: var(--text-tertiary); }
.sl-desc { font-size: var(--text-sm); color: var(--text-secondary); }
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
