<template>
  <NeDialog v-model="isOpen" :title="dialogTitle" size="md">
    <form @submit.prevent="handleSubmit" class="entity-form">
      <div class="form-group">
        <label class="form-label">名称 *</label>
        <input v-model="form.name" class="form-input" :placeholder="namePlaceholder" required />
      </div>
      <div class="form-group">
        <label class="form-label">摘要</label>
        <input v-model="form.summary" class="form-input" :placeholder="summaryPlaceholder" />
      </div>
      <div class="form-group">
        <label class="form-label">描述</label>
        <textarea v-model="form.description" class="form-textarea" :placeholder="descriptionPlaceholder" rows="4"></textarea>
      </div>
      <div v-if="error" class="form-error">{{ error }}</div>
    </form>
    <template #footer>
      <button class="btn-secondary" @click="close">取消</button>
      <button class="btn-primary" :disabled="submitting" @click="handleSubmit">
        {{ submitting ? '保存中...' : (isEditing ? '更新' : '创建') }}
      </button>
    </template>
  </NeDialog>
</template>

<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import NeDialog from './NeDialog.vue'

const props = defineProps<{
  modelValue: boolean
  title?: string
  entityType?: string
  editData?: { id?: string; name?: string; summary?: string; description?: string } | null
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  submit: [data: { name: string; summary?: string; description?: string }]
}>()

const isOpen = computed({
  get: () => props.modelValue,
  set: (v) => emit('update:modelValue', v)
})

const form = ref({
  name: '',
  summary: '',
  description: '',
})

const error = ref('')
const submitting = ref(false)

const isEditing = computed(() => !!props.editData?.id)
const dialogTitle = computed(() => {
  if (props.title) return props.title
  if (isEditing.value) return `编辑${props.entityType || '实体'}`
  return `新建${props.entityType || '实体'}`
})
const namePlaceholder = computed(() => `请输入${props.entityType || '实体'}名称`)
const summaryPlaceholder = computed(() => `请输入${props.entityType || '实体'}摘要`)
const descriptionPlaceholder = computed(() => `请输入${props.entityType || '实体'}详细描述`)

watch(() => props.modelValue, (v) => {
  if (v) {
    if (props.editData) {
      form.value = {
        name: props.editData.name || '',
        summary: props.editData.summary || '',
        description: props.editData.description || '',
      }
    } else {
      form.value = { name: '', summary: '', description: '' }
    }
    error.value = ''
  }
})

function close() {
  emit('update:modelValue', false)
}

async function handleSubmit() {
  if (!form.value.name.trim()) {
    error.value = '请输入名称'
    return
  }
  submitting.value = true
  error.value = ''
  try {
    emit('submit', {
      name: form.value.name.trim(),
      summary: form.value.summary.trim() || undefined,
      description: form.value.description.trim() || undefined,
    })
    close()
  } catch (e: any) {
    error.value = e.message || '操作失败'
  } finally {
    submitting.value = false
  }
}
</script>

<style scoped>
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
.form-error { color: var(--color-error); font-size: var(--text-xs); padding: var(--space-2); background: var(--color-error-subtle); border-radius: var(--radius-sm); }
.btn-primary {
  padding: var(--space-2) var(--space-4);
  background: var(--color-primary);
  border: none;
  color: white;
  border-radius: var(--radius-sm);
  font-size: var(--text-sm);
  cursor: pointer;
  transition: background var(--transition-fast);
}
.btn-primary:hover { background: var(--color-primary-hover); }
.btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }
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
