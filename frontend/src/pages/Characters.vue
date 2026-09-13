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
      <User class="empty-icon" :size="40" />
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
    <template v-if="showDetail && viewingEntity">
      <ProfilePanel
        ref="profilePanelRef"
        title="角色设定"
        :subtitle="viewingEntity.name"
        :groups="profileGroups"
        :model-value="panelValue"
        :saving="savingProfile"
        extra-label="编辑基础信息"
        @save="saveProfile"
        @close="showDetail = false"
        @extra="openBaseEdit()"
      />
      <ProfilePanel
        ref="statePanelRef"
        title="当前状态"
        :groups="stateGroups"
        :model-value="stateForm"
        :saving="savingState"
        :show-close="false"
        @save="saveState"
      />
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useWorldStore } from '@/stores/world'
import EntityCard from '@/components/ui/EntityCard.vue'
import EntityDialog from '@/components/ui/EntityDialog.vue'
import ProfilePanel, { type ProfileGroup } from '@/components/ui/ProfilePanel.vue'
import { ARC_STAGES_FIELD } from '@/utils/arcStages'
import { characterApi } from '@/api/character'
import type { Entity } from '@/types'
import type { CharacterProfile, CharacterState } from '@/types/character'
import {
  AGE_OPTIONS,
  GENDER_OPTIONS,
  ROLE_OPTIONS,
  emptyProfileForm,
  formToProfile,
  profileToForm,
  type ProfileForm,
} from '@/utils/characterProfile'
import { User } from 'lucide-vue-next'

const worldStore = useWorldStore()

const showCreateDialog = ref(false)
const showDialog = ref(false)
const editingEntity = ref<Entity | null>(null)
const showDetail = ref(false)
const viewingEntity = ref<Entity | null>(null)

// 表单模型 ProfileForm、枚举选项与双向映射（profileToForm / formToProfile）
// 统一放在 utils/characterProfile.ts，那里有单测覆盖这层「表单 ↔ 后端契约」的转换。

/** 字段按语义分组：平铺成一排时，读的人抓不到结构 */
const profileGroups: ProfileGroup[] = [
  {
    title: '基本',
    fields: [
      { key: 'name', label: '真名' },
      { key: 'aliases', label: '别名', hint: '多个别名用逗号分隔' },
      { key: 'age_range', label: '年龄段', options: AGE_OPTIONS },
      { key: 'gender', label: '性别', options: GENDER_OPTIONS },
      { key: 'identity', label: '身份' },
      { key: 'social_position_rank', label: '社会地位' },
    ],
  },
  {
    title: '性格与外貌',
    fields: [
      { key: 'appearance', label: '外貌', multiline: true },
      { key: 'core_personality', label: '核心性格', multiline: true },
      { key: 'values', label: '价值观', multiline: true },
    ],
  },
  {
    title: '来历与定位',
    fields: [
      { key: 'background_origin', label: '背景', multiline: true },
      { key: 'role_in_story', label: '故事功能位', options: ROLE_OPTIONS },
    ],
  },
  {
    title: '驱动力与冲突',
    fields: [
      {
        key: 'drive',
        label: '驱动力',
        shape: 'object',
        readOnly: true,
        subFields: [
          { key: 'motivation', label: '核心动机' },
          { key: 'primary_goal', label: '首要目标' },
          { key: 'desire', label: '欲望' },
          { key: 'fear', label: '恐惧' },
          { key: 'weakness', label: '弱点' },
          { key: 'contradiction', label: '内在矛盾' },
          { key: 'hidden_goal', label: '隐藏目的' },
          { key: 'long_term', label: '长期目标' },
          { key: 'current', label: '当前目标' },
          { key: 'immediate', label: '眼前目标' },
        ],
      },
      {
        key: 'conflicts',
        label: '冲突',
        shape: 'object-list',
        readOnly: true,
        primaryKey: 'description',
        badgeKey: 'conflict_type',
        badgeLabels: {
          Internal: '内在',
          External: '外在',
          Relationship: '关系',
          Ideology: '理念',
        },
        subFields: [
          { key: 'phase', label: '生效阶段' },
          { key: 'resolution_status', label: '状态' },
        ],
      },
      {
        key: 'secrets',
        label: '秘密',
        shape: 'object-list',
        readOnly: true,
        primaryKey: 'content',
        badgeKey: 'importance',
        badgePrefix: '重要度 ',
      },
    ],
  },
  {
    title: '能力与弧光',
    fields: [
      {
        key: 'capabilities',
        label: '能力边界',
        shape: 'object',
        readOnly: true,
        subFields: [
          { key: 'skills', label: '擅长' },
          { key: 'limitations', label: '限制' },
        ],
      },
      {
        key: 'arc_potential',
        label: '弧光潜力',
        shape: 'object',
        readOnly: true,
        subFields: [
          { key: 'starting_state', label: '起点' },
          { key: 'possible_change', label: '变化方向' },
          { key: 'resistance', label: '阻力' },
        ],
      },
      ARC_STAGES_FIELD,
    ],
  },
]

/**
 * 面板要展示的值：档案里既有「扁平可编辑字段」（真名 / 身份…），
 * 也有「结构化扩展字段」（驱动力 / 冲突 / 秘密 / 能力 / 弧光）。
 * 后者不在表单模型里，直接取自后端返回的原始档案。
 */
const panelValue = computed(() => ({
  ...(rawProfile.value ?? {}),
  ...profileForm.value,
}))

/** 人物状态：角色在故事当下所处的处境 */
const stateGroups: ProfileGroup[] = [
  {
    title: '处境',
    fields: [
      { key: 'location', label: '所在地' },
      { key: 'physical_state', label: '身体状态', multiline: true },
      { key: 'mental_state', label: '心理状态', multiline: true },
    ],
  },
  {
    title: '资源与关系',
    fields: [
      { key: 'resource_state', label: '资源状态', multiline: true },
      { key: 'social_state', label: '社会状态', multiline: true },
    ],
  },
]

const profilePanelRef = ref<InstanceType<typeof ProfilePanel> | null>(null)
const statePanelRef = ref<InstanceType<typeof ProfilePanel> | null>(null)

const profileForm = ref<ProfileForm>(emptyProfileForm())
const stateForm = ref<Partial<CharacterState>>({})
const savingProfile = ref(false)
const savingState = ref(false)

/** 原始档案：提交时靠它保留 social_position 里没在表单上暴露的子字段 */
const rawProfile = ref<CharacterProfile | null>(null)

async function openDetail(entity: Entity) {
  viewingEntity.value = entity
  showDetail.value = true
  worldStore.error = ''
  try {
    // 后端在"还没有档案/状态"时返回 null，这是正常语义，不是错误
    rawProfile.value = (await characterApi.getProfile(entity.id)) ?? null
    profileForm.value = rawProfile.value ? profileToForm(rawProfile.value) : emptyProfileForm()
    const st = (await characterApi.getState(entity.id)) ?? ({} as Partial<CharacterState>)
    stateForm.value = st
  } catch (e) {
    rawProfile.value = null
    profileForm.value = emptyProfileForm()
    stateForm.value = {}
    worldStore.error = `加载人物档案失败：${(e as Error).message}`
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
  savingProfile.value = true
  worldStore.error = ''
  try {
    const saved = await characterApi.updateProfile(
      viewingEntity.value.id,
      // ProfilePanel 对各类档案通用，以 Record<string, unknown> 传值；
      // 这里收窄回人物档案表单的类型
      formToProfile(value as unknown as ProfileForm, rawProfile.value),
    )
    // 用后端回写的真实值刷新表单与基准，避免本地状态与库中不一致
    rawProfile.value = saved
    profileForm.value = profileToForm(saved)
    // 保存成功才退出编辑态；失败时保留输入，避免白填一遍
    profilePanelRef.value?.finishEdit()
  } catch (e) {
    worldStore.error = `保存人物档案失败：${(e as Error).message}`
  } finally {
    savingProfile.value = false
  }
}

async function saveState(value: Record<string, unknown>) {
  if (!viewingEntity.value) return
  savingState.value = true
  worldStore.error = ''
  try {
    const saved = await characterApi.updateState(viewingEntity.value.id, {
      ...value,
      // extra 是历史遗留字段，界面上不再暴露；原样带回，避免被整行覆盖写入清空
      extra: stateForm.value.extra ?? null,
    })
    stateForm.value = saved
    statePanelRef.value?.finishEdit()
  } catch (e) {
    worldStore.error = `保存人物状态失败：${(e as Error).message}`
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

</style>
