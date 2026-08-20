<script setup lang="ts">
import { ref } from 'vue'
import { proposalApi } from '@/api/proposal'
import { useProposalStore } from '@/stores/proposal'
import type { ExtractionResult } from '@/types'

const props = defineProps<{ projectId: string }>()

const text = ref('')
const loading = ref(false)
const error = ref<string | null>(null)
const result = ref<ExtractionResult | null>(null)
const proposalStore = useProposalStore()

async function runExtract() {
  if (!text.value.trim()) {
    error.value = '请先粘贴文本'
    return
  }
  loading.value = true
  error.value = null
  result.value = null
  try {
    const r = await proposalApi.extract(props.projectId, text.value)
    result.value = r
    // 抽取成功后刷新提案列表，展示新创建的草稿
    proposalStore.fetchProposals(props.projectId)
  } catch (e: any) {
    error.value = e.message
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div class="extraction-panel">
    <h3>文本抽取（AI → 世界模型）</h3>
    <p class="hint">粘贴一段小说正文，AI 会抽取实体与关系并生成待审提案（不直接入库）。</p>
    <textarea v-model="text" rows="8" placeholder="在此粘贴正文…" :disabled="loading" />
    <div class="actions">
      <button :disabled="loading" @click="runExtract">
        {{ loading ? '抽取中…' : '抽取实体 / 关系' }}
      </button>
    </div>
    <p v-if="error" class="error">{{ error }}</p>

    <div v-if="result" class="result">
      <h4>候选实体（{{ result.entities.length }}）</h4>
      <ul>
        <li v-for="(e, i) in result.entities" :key="i">
          <strong>{{ e.name }}</strong>
          <span class="tag">{{ e.entity_type }}</span>
          <span v-if="e.summary" class="summary">— {{ e.summary }}</span>
        </li>
      </ul>
      <h4>候选关系（{{ result.relations.length }}）</h4>
      <ul>
        <li v-for="(r, i) in result.relations" :key="i">
          {{ r.from }}
          <span class="tag">{{ r.relation_type || '未知' }}</span>
          {{ r.to }}
          <span v-if="r.description" class="summary">— {{ r.description }}</span>
        </li>
      </ul>
      <p class="hint">已生成提案草稿，请到「Proposals」页确认后提交到 World Canon。</p>
    </div>
  </div>
</template>

<style scoped>
.extraction-panel {
  padding: 1rem;
  max-width: 720px;
}
textarea {
  width: 100%;
  font-family: inherit;
  padding: 0.5rem;
}
.tag {
  background: #2d6cdf;
  color: #fff;
  border-radius: 4px;
  padding: 0 6px;
  font-size: 12px;
  margin: 0 4px;
}
.summary {
  color: #888;
}
.error {
  color: #c0392b;
}
.actions {
  margin: 0.5rem 0;
}
</style>
