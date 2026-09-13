<template>
  <div class="locations-page">
    <div class="page-header">
      <h1 class="page-title">地点</h1>
      <button class="btn-primary" @click="openCreate()">+ 新建地点</button>
    </div>
    <div v-if="worldStore.error" class="error-banner">{{ worldStore.error }}</div>
    <div v-if="worldStore.locations.length" class="entity-grid">
      <EntityCard
        v-for="loc in worldStore.locations"
        :key="loc.id"
        :entity="loc"
        type="Location"
        @click="openDetail(loc)"
        @delete="handleDelete(loc)"
      />
    </div>
    <div v-else class="empty-state">
      <MapPin class="empty-icon" :size="40" />
      <span class="empty-text">暂无地点，点击上方按钮创建</span>
    </div>

    <!-- Base info (name/summary/description) dialog -->
    <EntityDialog
      v-model="showDialog"
      :entity-type="'地点'"
      :edit-data="editingEntity"
      @submit="handleSubmit"
    />

    <!-- Location design detail panel -->
    <ProfilePanel
      v-if="showDetail && viewingEntity"
      ref="panelRef"
      title="地点设计档案"
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
import { locationProfileApi } from '@/api/character'
import ProfilePanel, { type ProfileGroup } from '@/components/ui/ProfilePanel.vue'
import { ARC_STAGES_FIELD } from '@/utils/arcStages'
import type { Entity, LocationProfile } from '@/types'
import { MapPin } from 'lucide-vue-next'

const worldStore = useWorldStore()

const showDialog = ref(false)
const editingEntity = ref<Entity | null>(null)
const showDetail = ref(false)
const viewingEntity = ref<Entity | null>(null)
const profileForm = ref<Partial<LocationProfile>>({})
const saving = ref(false)

/** 字段按语义分组：12 个字段平铺成一坨时，读的人抓不到结构 */
const profileGroups: ProfileGroup[] = [
  {
    title: '地理与人口',
    fields: [
      { key: 'location_type', label: '地点类型' },
      { key: 'size', label: '规模' },
      { key: 'climate', label: '气候' },
      { key: 'era', label: '纪元' },
      { key: 'accessibility', label: '可达性' },
      { key: 'population', label: '人口' },
      { key: 'geography', label: '地理', multiline: true },
    ],
  },
  {
    title: '面貌与经济',
    fields: [
      { key: 'appearance', label: '外貌', multiline: true },
      { key: 'economy', label: '经济', multiline: true },
    ],
  },
  {
    title: '阶段弧线',
    fields: [ARC_STAGES_FIELD],
  },
  {
    title: '规则、历史与叙事',
    fields: [
      { key: 'rules', label: '规则', multiline: true },
      { key: 'history', label: '历史', multiline: true },
      { key: 'narrative_usage', label: '叙事用途', multiline: true },
    ],
  },
]

const panelRef = ref<InstanceType<typeof ProfilePanel> | null>(null)

async function openDetail(entity: Entity) {
  viewingEntity.value = entity
  showDetail.value = true
  worldStore.error = ''
  try {
    // 后端在"还没有档案"时返回 null，这是正常语义，不是错误
    profileForm.value = (await locationProfileApi.get(entity.id)) ?? ({} as Partial<LocationProfile>)
  } catch (e) {
    profileForm.value = {}
    worldStore.error = `加载地点档案失败：${(e as Error).message}`
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
    profileForm.value = await locationProfileApi.upsert(
      viewingEntity.value.id,
      value as Partial<LocationProfile>,
    )
    // 保存成功才退出编辑态；失败时保留输入，避免白填一遍
    panelRef.value?.finishEdit()
  } catch (e) {
    worldStore.error = `保存地点档案失败：${(e as Error).message}`
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
    await worldStore.updateLocation(editingEntity.value.id, data)
    if (viewingEntity.value?.id === editingEntity.value.id) {
      viewingEntity.value = { ...viewingEntity.value, ...data }
    }
  } else {
    await worldStore.createLocation(worldId, data)
  }
  editingEntity.value = null
}

async function handleDelete(entity: Entity) {
  if (!confirm(`确认删除「${entity.name}」？此操作不可撤销。`)) return
  await worldStore.deleteLocation(entity.id)
  if (viewingEntity.value?.id === entity.id) {
    showDetail.value = false
    viewingEntity.value = null
  }
}
</script>

<style scoped>
.locations-page { height: 100%; overflow-y: auto; padding: var(--space-6) var(--space-8); }
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
