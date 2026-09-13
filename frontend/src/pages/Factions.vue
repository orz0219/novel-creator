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
      <Swords class="empty-icon" :size="40" />
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
    <ProfilePanel
      v-if="showDetail && viewingEntity"
      ref="panelRef"
      title="势力设计档案"
      :subtitle="viewingEntity.name"
      :groups="profileGroups"
      :model-value="profileForm"
      :saving="saving"
      extra-label="编辑基础信息"
      @save="saveProfile"
      @close="showDetail = false"
      @extra="openBaseEdit()"
    />
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useWorldStore } from '@/stores/world'
import EntityCard from '@/components/ui/EntityCard.vue'
import EntityDialog from '@/components/ui/EntityDialog.vue'
import { factionProfileApi } from '@/api/character'
import ProfilePanel, { type ProfileGroup } from '@/components/ui/ProfilePanel.vue'
import { ARC_STAGES_FIELD } from '@/utils/arcStages'
import type { Entity, FactionProfile } from '@/types'
import { Swords } from 'lucide-vue-next'

const worldStore = useWorldStore()

const showDialog = ref(false)
const editingEntity = ref<Entity | null>(null)
const showDetail = ref(false)
const viewingEntity = ref<Entity | null>(null)
const profileForm = ref<Partial<FactionProfile>>({})
const saving = ref(false)

const panelRef = ref<InstanceType<typeof ProfilePanel> | null>(null)

/**
 * 字段按语义分组。
 * 11 个字段平铺成一坨时，读的人抓不到结构——「目标/领袖」和「内部矛盾/秘密」
 * 视觉权重完全一样，等于没有层级。
 */
const profileGroups: ProfileGroup[] = [
  {
    title: '基本',
    fields: [
      { key: 'goals', label: '目标', multiline: true },
      { key: 'leader', label: '领袖' },
      { key: 'values', label: '价值观', multiline: true },
    ],
  },
  {
    title: '实力',
    fields: [
      { key: 'resources', label: '资源', multiline: true },
      { key: 'territory', label: '领地', multiline: true },
      { key: 'members', label: '成员', multiline: true },
    ],
  },
  {
    title: '关系',
    fields: [
      { key: 'enemies', label: '敌人', multiline: true },
      { key: 'allies', label: '盟友', multiline: true },
    ],
  },
  {
    title: '阶段弧线',
    fields: [ARC_STAGES_FIELD],
  },
  {
    title: '隐情',
    fields: [
      { key: 'internal_conflicts', label: '内部矛盾', multiline: true },
      { key: 'secrets', label: '秘密', multiline: true },
      { key: 'modus_operandi', label: '行事风格', multiline: true },
    ],
  },
]

async function openDetail(entity: Entity) {
  viewingEntity.value = entity
  showDetail.value = true
  worldStore.error = ''
  try {
    // 后端在"还没有档案"时返回 null，这是正常语义，不是错误
    profileForm.value = (await factionProfileApi.get(entity.id)) ?? ({} as Partial<FactionProfile>)
  } catch (e) {
    profileForm.value = {}
    worldStore.error = `加载势力档案失败：${(e as Error).message}`
  }
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

async function saveProfile(value: Record<string, unknown>) {
  if (!viewingEntity.value) return
  saving.value = true
  worldStore.error = ''
  try {
    profileForm.value = await factionProfileApi.upsert(
      viewingEntity.value.id,
      value as Partial<FactionProfile>,
    )
    // 保存成功才退出编辑态；失败时保留用户输入，避免白填一遍
    panelRef.value?.finishEdit()
  } catch (e) {
    worldStore.error = `保存势力档案失败：${(e as Error).message}`
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

</style>
