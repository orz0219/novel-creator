<template>
  <div class="factions-page">
    <div class="page-header">
      <h1 class="page-title">势力</h1>
      <button class="btn-primary" @click="openCreate()">+ 新建势力</button>
    </div>
    <div v-if="worldStore.error" class="error-banner">{{ worldStore.error }}</div>
    <div v-if="worldStore.factions.length" class="entity-grid">
      <EntityCard
        v-for="fac in worldStore.factions"
        :key="fac.id"
        :entity="fac"
        type="Faction"
        @click="openDetail(fac)"
        @delete="handleDelete(fac)"
      />
    </div>
    <div v-else class="empty-state">
      <span class="empty-icon">⚔️</span>
      <span class="empty-text">暂无势力，点击上方按钮创建</span>
    </div>

    <!-- Base info (name/summary/description) dialog -->
    <EntityDialog
      v-model="showDialog"
      :entity-type="'势力'"
      :edit-data="editingEntity"
      @submit="handleSubmit"
    />

    <!-- Faction design detail panel -->
    <div v-if="showDetail && viewingEntity" class="detail-panel">
      <div class="detail-header">
        <span class="detail-title">势力设计档案</span>
        <span class="detail-subtitle">{{ viewingEntity.name }}</span>
        <button class="btn-ghost" @click="openBaseEdit()">编辑基础信息</button>
        <button class="btn-ghost" @click="showDetail = false">收起</button>
      </div>

      <div class="detail-section-head">
        <h3 class="detail-section-title">设计字段</h3>
        <button class="btn-save" :disabled="saving" @click="saveProfile()">
          {{ saving ? '保存中…' : '保存档案' }}
        </button>
      </div>

      <div class="profile-grid">
        <div class="form-group" v-for="f in profileFields" :key="f.key">
          <label class="form-label">{{ f.label }}</label>
          <textarea
            v-if="f.textarea"
            v-model="profileForm[f.key]"
            class="form-textarea"
            rows="2"
          ></textarea>
          <input v-else v-model="profileForm[f.key]" class="form-input" type="text" />
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useWorldStore } from '@/stores/world'
import EntityCard from '@/components/ui/EntityCard.vue'
import EntityDialog from '@/components/ui/EntityDialog.vue'
import { factionProfileApi } from '@/api/character'
import type { Entity, FactionProfile } from '@/types'

const worldStore = useWorldStore()

const showDialog = ref(false)
const editingEntity = ref<Entity | null>(null)
const showDetail = ref(false)
const viewingEntity = ref<Entity | null>(null)
const profileForm = ref<Partial<FactionProfile>>({})
const saving = ref(false)

type FacField = { key: keyof FactionProfile; label: string; textarea?: boolean }
const profileFields: FacField[] = [
  { key: 'goals', label: '目标', textarea: true },
  { key: 'leader', label: '领袖' },
  { key: 'values', label: '价值观', textarea: true },
  { key: 'resources', label: '资源', textarea: true },
  { key: 'territory', label: '领地', textarea: true },
  { key: 'members', label: '成员', textarea: true },
  { key: 'enemies', label: '敌人', textarea: true },
  { key: 'allies', label: '盟友', textarea: true },
  { key: 'internal_conflicts', label: '内部冲突', textarea: true },
  { key: 'secrets', label: '秘密', textarea: true },
  { key: 'modus_operandi', label: '行事风格', textarea: true },
]

async function openDetail(entity: Entity) {
  viewingEntity.value = entity
  showDetail.value = true
  profileForm.value =
    (await factionProfileApi.get(entity.id).catch(() => null)) ?? ({} as Partial<FactionProfile>)
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
  saving.value = true
  try {
    await factionProfileApi.upsert(viewingEntity.value.id, profileForm.value)
  } catch (e: any) {
    worldStore.error = e?.message || '保存势力档案失败'
  } finally {
    saving.value = false
  }
}

async function handleSubmit(data: { name: string; summary?: string; description?: string }) {
  const worldId = worldStore.currentWorld?.id
  if (!worldId) {
    worldStore.error = '未找到世界数据'
    return
  }
  if (editingEntity.value) {
    await worldStore.updateFaction(editingEntity.value.id, data)
    if (viewingEntity.value?.id === editingEntity.value.id) {
      viewingEntity.value = { ...viewingEntity.value, ...data }
    }
  } else {
    await worldStore.createFaction(worldId, data)
  }
  editingEntity.value = null
}

async function handleDelete(entity: Entity) {
  if (!confirm(`确认删除「${entity.name}」？此操作不可撤销。`)) return
  await worldStore.deleteFaction(entity.id)
  if (viewingEntity.value?.id === entity.id) {
    showDetail.value = false
    viewingEntity.value = null
  }
}
</script>

<style scoped>
.factions-page { height: 100%; overflow-y: auto; padding: var(--space-6) var(--space-8); }
.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--space-6); }
.page-title { font-size: var(--text-2xl); font-weight: 700; font-family: var(--font-serif); }
.btn-primary { padding: var(--space-2) var(--space-4); background: var(--color-primary); border: none; color: white; border-radius: var(--radius-sm); font-size: var(--text-sm); cursor: pointer; }
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

.detail-section-head { display: flex; align-items: center; justify-content: space-between; margin-bottom: var(--space-3); }
.detail-section-title { font-size: var(--text-sm); font-weight: 600; color: var(--text-primary); }
.btn-save { padding: var(--space-1) var(--space-3); background: var(--color-primary); border: none; color: white; border-radius: var(--radius-sm); font-size: var(--text-xs); cursor: pointer; }
.btn-save:disabled { opacity: 0.6; cursor: default; }

.profile-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap: var(--space-4); }
.form-group { display: flex; flex-direction: column; gap: var(--space-1); }
.form-label { font-size: var(--text-sm); font-weight: 500; color: var(--text-secondary); }
.form-input, .form-textarea {
  padding: var(--space-2) var(--space-3);
  background: var(--bg-base);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: var(--text-sm);
  outline: none;
  font-family: inherit;
}
.form-textarea { resize: vertical; }
.form-input:focus, .form-textarea:focus { border-color: var(--color-primary); }
</style>
