import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { narrativeApi } from '@/api/story'

export const useEditorStore = defineStore('editor', () => {
  const currentSceneId = ref<string | null>(null)
  const content = ref('')
  const isDirty = ref(false)
  const wordCount = computed(() => {
    if (!content.value) return 0
    return content.value.replace(/\s/g, '').length
  })
  const charCount = computed(() => content.value.length)

  // 真实内容：从叙事节点读取（content 字段由后端持久化）。
  async function loadScene(sceneId: string) {
    currentSceneId.value = sceneId
    try {
      const node = await narrativeApi.getNode(sceneId)
      content.value = node.content || ''
    } catch {
      // 载入失败（如网络抖动）时保留当前内容，避免误清空正在编辑的文本。
    }
    isDirty.value = false
  }

  function updateContent(text: string) {
    content.value = text
    isDirty.value = true
  }

  // 保存：PUT /narrative/{id} 写入 content，刷新/重开不再丢失。
  async function saveContent() {
    if (currentSceneId.value) {
      await narrativeApi.updateNode(currentSceneId.value, { content: content.value })
      isDirty.value = false
    }
  }

  return {
    currentSceneId, content, isDirty, wordCount, charCount,
    loadScene, updateContent, saveContent,
  }
})
