import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

export const useEditorStore = defineStore('editor', () => {
  const currentSceneId = ref<string | null>(null)
  const content = ref('')
  const isDirty = ref(false)
  const wordCount = computed(() => {
    if (!content.value) return 0
    return content.value.replace(/\s/g, '').length
  })
  const charCount = computed(() => content.value.length)

  const sceneContents: Record<string, string> = {
    'scene-1': [
      '林凡站在一处隐蔽的山洞前，洞口被茂密的藤蔓遮掩，若非苏晚晴指引，他绝不可能找到这里。',
      '',
      '「就是这里？」林凡压低声音问道。',
      '',
      '苏晚晴点了点头，她的目光中闪过一丝复杂的神色：「地下遗迹的入口就在洞内深处。我在古籍中找到过记载，这里曾是远古天玄宗的炼器坊。」',
      '',
      '林凡深吸一口气，灵气在体内流转，他能感觉到一股若有若无的威压从洞内传出。这种感觉，和他在古井旁感受到的如出一辙。',
      '',
      '「小心些，」苏晚晴提醒道，「遗迹内可能还有残存的阵法。」',
      '',
      '两人一前一后走入山洞。洞内漆黑一片，但林凡运转灵力后，双眼泛起淡淡的光芒，周围的景物清晰可见。',
      '',
      '山洞越来越深，空气中的灵气浓度也在逐渐升高。终于，他们来到了一扇巨大的石门前。',
      '',
      '石门上刻满了密密麻麻的符文，中央有一个拳头大小的凹槽，形状恰好和林凡怀中的黑色令牌吻合。',
    ].join('\n'),
    'scene-2': ''
  }

  function loadScene(sceneId: string) {
    currentSceneId.value = sceneId
    content.value = sceneContents[sceneId] || ''
    isDirty.value = false
  }

  function updateContent(text: string) {
    content.value = text
    isDirty.value = true
  }

  function saveContent() {
    if (currentSceneId.value) {
      sceneContents[currentSceneId.value] = content.value
      isDirty.value = false
    }
  }

  return {
    currentSceneId, content, isDirty, wordCount, charCount, sceneContents,
    loadScene, updateContent, saveContent,
  }
})
