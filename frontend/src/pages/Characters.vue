<template>
  <div class="characters-page">
    <div class="page-header">
      <h1 class="page-title">人物</h1>
      <button class="btn-primary" @click="openCreate()">+ 新建人物</button>
    </div>
    <div v-if="worldStore.error" class="error-banner">{{ worldStore.error }}</div>
    <div v-if="worldStore.characters.length" class="entity-grid">
      <EntityCard
        v-for="char in worldStore.characters"
        :key="char.id"
        :entity="char"
        type="Character"
        @click="openDetail(char)"
        @delete="handleDelete(char)"
      />
    </div>
    <div v-else class="empty-state">
      <span class="empty-icon">👤</span>
      <span class="empty-text">暂无人物，点击上方按钮创建</span>
    </div>

    <!-- Base info (name/summary/description) dialog -->
    <EntityDialog
      v-model="showDialog"
      :entity-type="'人物'"
      :edit-data="editingEntity"
      @submit="handleSubmit"
    />

    <!-- Character design detail panel (profile + state) -->
    <div v-if="showDetail && viewingEntity" class="detail-panel">
      <div class="detail-header">
        <span class="detail-title">人物档案</span>
        <span class="detail-subtitle">{{ viewingEntity.name }}</span>
        <button class="btn-ghost" @click="openBaseEdit()">编辑基础信息</button>
        <button class="btn-ghost" @click="showDetail = false">收起</button>
      </div>

      <section class="detail-section">
        <div class="detail-section-head">
          <h3 class="detail-section-title">角色设定</h3>
          <button class="btn-save" :disabled="savingProfile" @click="saveProfile()">
            {{ savingProfile ? '保存中…' : '保存档案' }}
          </button>
        </div>
        <div class="field-grid">
          <div class="field" v-for="f in profileFields" :key="f.key">
            <label class="field-label">{{ f.label }}</label>
            <textarea
              v-if="f.textarea"
              v-model="profileForm[f.key]"
              class="field-textarea"
              rows="2"
            ></textarea>
            <input v-else v-model="profileForm[f.key]" class="field-input" type="text" />
          </div>
        </div>
      </section>

      <section class="detail-section">
        <div class="detail-section-head">
          <h3 class="detail-section-title">当前状态</h3>
          <button class="btn-save" :disabled="savingState" @click="saveState()">
            {{ savingState ? '保存中…' : '保存状态' }}
          </button>
        </div>
        <div class="field-grid">
          <div class="field">
            <label class="field-label">所在地</label>
            <input v-model="stateForm.location" class="field-input" type="text" />
          </div>
          <div class="field">
            <label class="field-label">健康</label>
            <input v-model="stateForm.health" class="field-input" type="text" />
          </div>
          <div class="field">
            <label class="field-label">修为</label>
            <input v-model="stateForm.cultivation" class="field-input" type="text" />
          </div>
          <div class="field">
            <label class="field-label">财力</label>
            <input v-model="stateForm.money" class="field-input" type="text" />
          </div>
          <div class="field field-inline">
            <label class="field-label">通缉</label>
            <input v-model="stateForm.wanted" class="field-checkbox" type="checkbox" />
          </div>
          <div class="field field-wide">
            <label class="field-label">额外信息 (JSON)</label>
            <textarea v-model="stateExtraText" class="field-textarea" rows="2"></textarea>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useWorldStore } from '@/stores/world'
import EntityCard from '@/components/ui/EntityCard.vue'
import EntityDialog from '@/components/ui/EntityDialog.vue'
import { characterApi } from '@/api/character'
import type { Entity } from '@/types'
import type { CharacterProfile, CharacterState } from '@/types/character'

const worldStore = useWorldStore()

const showCreateDialog = ref(false)
const showDialog = ref(false)
const editingEntity = ref<Entity | null>(null)
const showDetail = ref(false)
const viewingEntity = ref<Entity | null>(null)

const profileForm = ref<Partial<CharacterProfile>>({})
const stateForm = ref<Partial<CharacterState>>({})
const stateExtraText = ref('')
const savingProfile = ref(false)
const savingState = ref(false)

const profileFields: { key: keyof CharacterProfile; label: string; textarea?: boolean }[] = [
  { key: 'real_name', label: '真名' },
  { key: 'nickname', label: '别名' },
  { key: 'age', label: '年龄' },
  { key: 'gender', label: '性别' },
  { key: 'identity', label: '身份' },
  { key: 'appearance', label: '外貌', textarea: true },
  { key: 'background', label: '背景', textarea: true },
  { key: 'social_status', label: '社会地位' },
  { key: 'core_personality', label: '核心性格', textarea: true },
  { key: 'values', label: '价值观' },
]

async function openDetail(entity: Entity) {
  viewingEntity.value = entity
  showDetail.value = true
  profileForm.value = (await characterApi.getProfile(entity.id).catch(() => null)) ?? ({} as Partial<CharacterProfile>)
  const st = (await characterApi.getState(entity.id).catch(() => null)) ?? ({} as Partial<CharacterState>)
  stateForm.value = st
  stateExtraText.value = st.extra ? JSON.stringify(st.extra, null, 2) : ''
}

function openCreate() {
  editingEntity.value = null
  showDialog.value = true
}

function openBaseEdit() {
  if (!viewingEntity.value) return
  editingEntity.value = viewingEntity.value
  showDialog.value = true
}

async function saveProfile() {
  if (!viewingEntity.value) return
  savingProfile.value = true
  try {
    await characterApi.updateProfile(viewingEntity.value.id, profileForm.value)
  } catch (e) {
    worldStore.error = '保存人物档案失败'
  } finally {
    savingProfile.value = false
  }
}

async function saveState() {
  if (!viewingEntity.value) return
  savingState.value = true
  try {
    let extra: unknown = null
    if (stateExtraText.value.trim()) {
      extra = JSON.parse(stateExtraText.value)
    }
    stateForm.value.extra = extra
    await characterApi.updateState(viewingEntity.value.id, stateForm.value)
  } catch (e) {
    worldStore.error = '保存人物状态失败（请检查额外信息 JSON 格式）'
  } finally {
    savingState.value = false
  }
}

async function handleSubmit(data: { name: string; summary?: string; description?: string }) {
  const worldId = worldStore.currentWorld?.id
  if (!worldId) {
    worldStore.error = '未找到世界数据'
    return
  }

  if (editingEntity.value) {
    await worldStore.updateCharacter(editingEntity.value.id, data)
    if (viewingEntity.value?.id === editingEntity.value.id) {
      viewingEntity.value = { ...viewingEntity.value, ...data }
    }
  } else {
    await worldStore.createCharacter(worldId, data)
  }
  editingEntity.value = null
}

async function handleDelete(entity: Entity) {
  if (!confirm(`确认删除「${entity.name}」？此操作不可撤销。`)) return
  await worldStore.deleteCharacter(entity.id)
  if (viewingEntity.value?.id === entity.id) {
    showDetail.value = false
    viewingEntity.value = null
  }
}
</script>

<style scoped>
.characters-page { height: 100%; overflow-y: auto; padding: var(--space-6) var(--space-8); }
.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--space-6); }
.page-title { font-size: var(--text-2xl); font-weight: 700; font-family: var(--font-serif); }
.btn-primary {
  padding: var(--space-2) var(--space-4);
  background: var(--color-primary);
  border: none; color: white; border-radius: var(--radius-sm);
  font-size: var(--text-sm); cursor: pointer;
}
.btn-primary:hover { background: var(--color-primary-hover); }
.entity-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: var(--space-4); }
.empty-state { display: flex; flex-direction: column; align-items: center; justify-content: center; padding: var(--space-16); color: var(--text-tertiary); }
.empty-icon { font-size: 48px; margin-bottom: var(--space-4); }
.empty-text { font-size: var(--text-sm); }
.error-banner { padding: var(--space-3) var(--space-4); background: var(--color-error-subtle); color: var(--color-error); border-radius: var(--radius-sm); margin-bottom: var(--space-4); font-size: var(--text-sm); }

.detail-panel { margin-top: var(--space-6); padding: var(--space-6); background: var(--color-surface); border: 1px solid var(--color-border); border-radius: var(--radius-sm); }
.detail-header { display: flex; align-items: baseline; gap: var(--space-3); margin-bottom: var(--space-4); }
.detail-title { font-size: var(--text-sm); font-weight: 600; color: var(--color-primary); font-family: var(--font-serif); }
.detail-subtitle { font-size: var(--text-sm); color: var(--text-tertiary); }
.btn-ghost { margin-left: auto; padding: var(--space-1) var(--space-3); background: transparent; border: 1px solid var(--color-border); border-radius: var(--radius-sm); font-size: var(--text-xs); cursor: pointer; color: var(--text-secondary); }
.btn-ghost:last-child { margin-left: var(--space-2); }
.btn-ghost:hover { background: var(--color-surface-hover); }

.detail-section { margin-bottom: var(--space-6); }
.detail-section:last-child { margin-bottom: 0; }
.detail-section-head { display: flex; align-items: center; justify-content: space-between; margin-bottom: var(--space-3); }
.detail-section-title { font-size: var(--text-sm); font-weight: 600; color: var(--text-primary); }
.btn-save { padding: var(--space-1) var(--space-3); background: var(--color-primary); border: none; color: white; border-radius: var(--radius-sm); font-size: var(--text-xs); cursor: pointer; }
.btn-save:disabled { opacity: 0.6; cursor: default; }

.field-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap: var(--space-3); }
.field { display: flex; flex-direction: column; gap: var(--space-1); }
.field-wide { grid-column: 1 / -1; }
.field-inline { flex-direction: row; align-items: center; gap: var(--space-2); }
.field-label { font-size: var(--text-xs); color: var(--text-tertiary); }
.field-input, .field-textarea {
  width: 100%; padding: var(--space-2) var(--space-3);
  border: 1px solid var(--color-border); border-radius: var(--radius-sm);
  background: var(--color-bg); color: var(--text-primary); font-size: var(--text-sm);
  font-family: inherit;
}
.field-textarea { resize: vertical; line-height: 1.5; }
.field-checkbox { width: 18px; height: 18px; }
</style>
