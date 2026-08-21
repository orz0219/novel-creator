<template>
  <div class="knowledge-panel">
    <div class="panel-header">
      <span class="panel-title">知识状态</span>
      <span class="panel-subtitle">共 {{ knowledgeItems.length }} 条知识记录</span>
    </div>

    <div v-if="loading" class="state-box">加载中…</div>
    <div v-else-if="!knowledgeItems.length" class="state-box">暂无知识记录</div>

    <div v-else class="knowledge-list">
      <div v-for="item in knowledgeItems" :key="item.key" class="k-item">
        <div class="k-meta">
          <span class="k-char">{{ item.characterName }}</span>
        </div>
        <div v-if="item.entityId || item.entityType" class="k-line">
          <span v-if="item.entityId" class="k-tag">实体: {{ item.entityId }}</span>
          <span v-if="item.entityType" class="k-tag">类型: {{ item.entityType }}</span>
        </div>
        <div v-if="item.knowledgeType" class="k-line">
          <span class="k-tag">知识类型: {{ item.knowledgeType }}</span>
        </div>
        <div v-if="item.content" class="k-content">{{ item.content }}</div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useRoute } from 'vue-router'
import { useWorldStore } from '@/stores/world'
import { characterApi } from '@/api/character'
import { ref, onMounted, computed } from 'vue'

const route = useRoute()
const worldStore = useWorldStore()

const projectId = route.params.id as string
const worldId = computed(() => worldStore.currentWorld?.id ?? '')

interface KnowledgeItem {
  key: string
  characterName: string
  entityId?: string
  entityType?: string
  knowledgeType?: string
  content?: string
}

const knowledgeItems = ref<KnowledgeItem[]>([])
const loading = ref(true)

let seq = 0

onMounted(async () => {
  loading.value = true
  try {
    if (!worldStore.currentWorld) await worldStore.fetchWorld(projectId)
    if (worldId.value) await worldStore.fetchCharacters(worldId.value)

    const items: KnowledgeItem[] = []
    await Promise.all(
      worldStore.characters.map(async (char) => {
        try {
          const knowledge = await characterApi.getKnowledge(char.id)
          if (Array.isArray(knowledge)) {
            for (const entry of knowledge) {
              if (entry && typeof entry === 'object') {
                items.push({
                  key: `${char.id}-${seq++}`,
                  characterName: char.name,
                  entityId: entry.entity_id ?? entry.entityId,
                  entityType: entry.entity_type ?? entry.entityType,
                  knowledgeType: entry.knowledge_type ?? entry.knowledgeType,
                  content: entry.content ?? entry.summary,
                })
              }
            }
          }
        } catch {
          // skip characters whose knowledge fails to load
        }
      })
    )
    knowledgeItems.value = items
  } finally {
    loading.value = false
  }
})
</script>

<style scoped>
.knowledge-panel { display: flex; flex-direction: column; }
.panel-header { padding: var(--space-3) var(--space-4); border-bottom: 1px solid var(--border-muted); }
.panel-title { font-size: var(--text-sm); font-weight: 600; }
.panel-subtitle { font-size: var(--text-xs); color: var(--text-tertiary); margin-left: var(--space-2); }
.state-box { padding: var(--space-4); color: var(--text-tertiary); font-size: var(--text-sm); }
.knowledge-list { padding: var(--space-3); }
.k-item { padding: var(--space-2) 0; border-bottom: 1px solid var(--border-muted); }
.k-meta { display: flex; align-items: center; margin-bottom: var(--space-1); }
.k-char { font-size: var(--text-sm); font-weight: 600; color: var(--color-primary); }
.k-line { display: flex; flex-wrap: wrap; gap: var(--space-2); margin-bottom: var(--space-1); }
.k-tag { font-size: var(--text-xs); color: var(--text-tertiary); }
.k-content { font-size: var(--text-sm); color: var(--text-secondary); display: block; }
</style>
