<template>
  <div class="items-page">
    <div class="page-header">
      <h1 class="page-title">物品</h1>
      <button class="btn-primary" @click="showCreateDialog = true">+ 新建物品</button>
    </div>
    <div v-if="items.length" class="entity-grid">
      <EntityCard
        v-for="item in items"
        :key="item.id"
        :entity="item"
        type="Item"
        @click="openEdit(item)"
        @delete="handleDelete(item)"
      />
    </div>
    <div v-else class="empty-state">
      <span class="empty-icon">📦</span>
      <span class="empty-text">暂无物品，点击上方按钮创建</span>
    </div>

    <NeDialog v-model="showDialog" :title="dialogTitle" size="md">
      <form @submit.prevent="handleSubmit" class="entity-form">
        <div class="form-group">
          <label class="form-label">名称 *</label>
          <input v-model="form.name" class="form-input" placeholder="请输入物品名称" required />
        </div>
        <div class="form-group">
          <label class="form-label">摘要</label>
          <input v-model="form.summary" class="form-input" placeholder="请输入物品摘要" />
        </div>
        <div class="form-group">
          <label class="form-label">描述</label>
          <textarea v-model="form.description" class="form-textarea" placeholder="请输入物品详细描述" rows="4"></textarea>
        </div>
        <div class="form-group">
          <label class="form-label">属性（attributes JSON）</label>
          <textarea v-model="attributesText" class="form-textarea attributes-editor" placeholder='{}' rows="8"></textarea>
          <span class="form-help">物品设计字段（JSON 格式）</span>
        </div>
        <div v-if="error" class="form-error">{{ error }}</div>
      </form>
      <template #footer>
        <button class="btn-secondary" @click="closeDialog">取消</button>
        <button class="btn-primary" :disabled="submitting" @click="handleSubmit">
          {{ submitting ? '保存中...' : (isEditing ? '更新' : '创建') }}
        </button>
      </template>
    </NeDialog>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { useWorldStore } from '@/stores/world'
import { entityApi } from '@/api/world'
import EntityCard from '@/components/ui/EntityCard.vue'
import NeDialog from '@/components/ui/NeDialog.vue'
import type { Entity } from '@/types'

const route = useRoute()
const worldStore = useWorldStore()
const items = ref<Entity[]>([])

const showCreateDialog = ref(false)
const showDialog = ref(false)
const editingEntity = ref<Entity | null>(null)

const attributesText = ref('{}')
const error = ref('')
const submitting = ref(false)

const isEditing = ref(false)
const dialogTitle = ref('新建物品')

const form = ref({
  name: '',
  summary: '',
  description: '',
})

async function loadItems() {
  const worldId = worldStore.currentWorld?.id
  if (!worldId) return
  items.value = await worldStore.fetchEntities(worldId, 'Item')
}

// ProjectLayout 异步解析 currentWorld；子页面 onMounted 可能早于 world 就绪，
// 用 watch 在 world 可用后再加载，兼顾深链直达与本页导航两种场景。
watch(() => worldStore.currentWorld?.id, (id) => { if (id) loadItems() }, { immediate: true })

function openEdit(entity: Entity) {
  editingEntity.value = entity
  isEditing.value = true
  dialogTitle.value = '编辑物品'
  form.value = {
    name: entity.name || '',
    summary: entity.summary || '',
    description: entity.description || '',
  }
  attributesText.value = JSON.stringify((entity.attributes as Record<string, unknown>) ?? {}, null, 2)
  error.value = ''
  showDialog.value = true
}

function resetForm() {
  editingEntity.value = null
  isEditing.value = false
  form.value = { name: '', summary: '', description: '' }
  attributesText.value = '{}'
  error.value = ''
}

function closeDialog() {
  showDialog.value = false
  resetForm()
}

async function handleSubmit() {
  const worldId = worldStore.currentWorld?.id
  if (!worldId) return

  if (!form.value.name.trim()) {
    error.value = '请输入名称'
    return
  }

  let parsed: Record<string, unknown> = {}
  try {
    parsed = JSON.parse(attributesText.value || '{}') as Record<string, unknown>
  } catch (e) {
    error.value = `属性 JSON 格式错误：${(e as Error).message}`
    return
  }

  submitting.value = true
  error.value = ''
  try {
    if (editingEntity.value) {
      await entityApi.update(editingEntity.value.id, {
        name: form.value.name.trim(),
        summary: form.value.summary.trim() || undefined,
        description: form.value.description.trim() || undefined,
        attributes: parsed,
      })
    } else {
      await worldStore.createEntity(worldId, {
        name: form.value.name.trim(),
        summary: form.value.summary.trim() || undefined,
        description: form.value.description.trim() || undefined,
      })
    }
    showDialog.value = false
    resetForm()
    await loadItems()
  } catch (e) {
    error.value = (e as Error).message || '操作失败'
  } finally {
    submitting.value = false
  }
}

async function handleDelete(entity: Entity) {
  if (!confirm(`确认删除「${entity.name}」？此操作不可撤销。`)) return
  await worldStore.deleteEntity(entity.id)
  items.value = items.value.filter(e => e.id !== entity.id)
}

watch(showCreateDialog, (v) => {
  if (v) {
    resetForm()
    dialogTitle.value = '新建物品'
    showDialog.value = true
    showCreateDialog.value = false
  }
})
</script>

<style scoped>
.items-page { height: 100%; overflow-y: auto; padding: var(--space-6) var(--space-8); }
.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--space-6); }
.page-title { font-size: var(--text-2xl); font-weight: 700; font-family: var(--font-serif); }
.btn-primary { padding: var(--space-2) var(--space-4); background: var(--color-primary); border: none; color: white; border-radius: var(--radius-sm); font-size: var(--text-sm); cursor: pointer; }
.btn-primary:hover { background: var(--color-primary-hover); }
.btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }
.btn-secondary { padding: var(--space-2) var(--space-4); background: transparent; border: 1px solid var(--border-default); color: var(--text-secondary); border-radius: var(--radius-sm); font-size: var(--text-sm); cursor: pointer; }
.btn-secondary:hover { border-color: var(--border-emphasis); color: var(--text-primary); }
.entity-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: var(--space-4); }
.empty-state { display: flex; flex-direction: column; align-items: center; justify-content: center; padding: var(--space-16); color: var(--text-tertiary); }
.empty-icon { font-size: 48px; margin-bottom: var(--space-4); }
.empty-text { font-size: var(--text-sm); }

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
  transition: border-color var(--transition-fast);
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
  transition: border-color var(--transition-fast);
}
.form-textarea:focus { border-color: var(--color-primary); }
.attributes-editor { font-family: var(--font-mono, monospace); }
.form-help { font-size: var(--text-xs); color: var(--text-tertiary); }
.form-error { color: var(--color-error); font-size: var(--text-xs); padding: var(--space-2); background: var(--color-error-subtle); border-radius: var(--radius-sm); }
</style>
