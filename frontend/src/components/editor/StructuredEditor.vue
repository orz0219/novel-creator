<template>
  <div class="structured-editor" ref="editorRef">
    <div class="editor-content">
      <div
        v-for="(block, index) in blocks"
        :key="block.id"
        class="editor-block"
        :class="[block.type, { active: activeBlockId === block.id }]"
        @click="setActiveBlock(block.id)"
      >
        <div class="block-gutter">
          <span class="block-type-icon">{{ blockIcons[block.type] }}</span>
        </div>
        <div
          class="block-content"
          contenteditable="true"
          :data-block-id="block.id"
          :data-block-type="block.type"
          @input="onBlockInput(index, $event)"
          @keydown="onBlockKeydown(index, $event)"
          @blur="onBlockBlur(index)"
          v-html="block.html"
        ></div>
      </div>
    </div>
    <div class="editor-footer">
      <button class="add-block-btn" @click="addBlock('paragraph')">+ 段落</button>
      <button class="add-block-btn" @click="addBlock('dialogue')">+ 对话</button>
      <button class="add-block-btn" @click="addBlock('action')">+ 动作</button>
      <button class="add-block-btn" @click="addBlock('narration')">+ 旁白</button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useRoute } from 'vue-router'
import { useWorldStore } from '@/stores/world'

interface EditorBlock {
  id: string
  type: 'paragraph' | 'dialogue' | 'action' | 'narration' | 'heading'
  content: string
  html: string
}

const props = defineProps<{
  modelValue: string
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
  (e: 'entity-click', name: string): void
}>()

const route = useRoute()
const worldStore = useWorldStore()
const projectId = route.params.id as string
const worldId = computed(() => worldStore.currentWorld?.id ?? '')

// DB-driven entity list: collect names from characters / locations / factions.
const entityNames = computed<string[]>(() => {
  return [
    ...worldStore.characters.map((c) => c.name),
    ...worldStore.locations.map((l) => l.name),
    ...worldStore.factions.map((f) => f.name),
  ].filter((name) => !!name)
})

const editorRef = ref<HTMLElement>()
const activeBlockId = ref<string | null>(null)
let blockCounter = 0

const blockIcons: Record<string, string> = {
  paragraph: '¶',
  dialogue: '💬',
  action: '⚡',
  narration: '📖',
  heading: '§',
}

const blocks = ref<EditorBlock[]>([])

// 由纯文本（按 \n 分行）重建块。异步 loadScene 后 modelValue 才就绪，必须可重建。
function rebuildBlocks(text: string) {
  if (text) {
    const lines = text.split('\n')
    blocks.value = lines.map(line => {
      const type = detectBlockType(line)
      return {
        id: 'block-' + blockCounter++,
        type,
        content: line,
        html: highlightEntities(line),
      }
    })
  } else {
    blocks.value = [{
      id: 'block-' + blockCounter++,
      type: 'paragraph',
      content: '',
      html: '',
    }]
  }
}

// Initialize blocks from modelValue
onMounted(async () => {
  if (!worldStore.currentWorld) await worldStore.fetchWorld(projectId)
  if (worldId.value) {
    await worldStore.fetchCharacters(worldId.value)
    await worldStore.fetchLocations(worldId.value)
    await worldStore.fetchFactions(worldId.value)
  }
  rebuildBlocks(props.modelValue)
})

// 外部载入（切换节点 / 异步拉取）时重建；自身输入触发的 modelValue 变化跳过，避免清空正在编辑的内容。
watch(() => props.modelValue, (val) => {
  if (val !== serialize()) {
    rebuildBlocks(val)
  }
})

function detectBlockType(text: string): EditorBlock['type'] {
  const trimmed = text.trim()
  if (trimmed.startsWith('「') || trimmed.startsWith('"') || trimmed.startsWith('"')) return 'dialogue'
  if (trimmed.startsWith('【') || trimmed.startsWith('---')) return 'heading'
  return 'paragraph'
}

function highlightEntities(text: string): string {
  // Highlight DB-driven entity names in insertion order (longer names first to avoid partial matches).
  const names = [...entityNames.value].sort((a, b) => b.length - a.length)
  let result = text
  for (const name of names) {
    if (!name) continue
    result = result.replace(
      new RegExp(escapeRegExp(name), 'g'),
      '<span class="entity-ref" data-entity-name="' + name + '">' + name + '</span>'
    )
  }
  return result
}

function escapeRegExp(str: string): string {
  return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

function setActiveBlock(id: string) {
  activeBlockId.value = id
}

function onBlockInput(index: number, event: Event) {
  const el = event.target as HTMLElement
  blocks.value[index].content = el.innerText
  blocks.value[index].html = highlightEntities(el.innerText)
  emitContent()
}

function onBlockKeydown(index: number, event: KeyboardEvent) {
  if (event.key === 'Enter' && !event.shiftKey) {
    event.preventDefault()
    addBlock('paragraph', index + 1)
  }
  if (event.key === 'Backspace' && blocks.value[index].content === '' && blocks.value.length > 1) {
    event.preventDefault()
    blocks.value.splice(index, 1)
    emitContent()
  }
}

function onBlockBlur(index: number) {
  // Clean up empty blocks
  if (!blocks.value[index].content.trim() && blocks.value.length > 1) {
    // Keep it for now, user might come back
  }
}

function addBlock(type: EditorBlock['type'], insertAt?: number) {
  const newBlock: EditorBlock = {
    id: 'block-' + blockCounter++,
    type,
    content: '',
    html: '',
  }
  if (insertAt !== undefined) {
    blocks.value.splice(insertAt, 0, newBlock)
  } else {
    blocks.value.push(newBlock)
  }
  // Focus the new block
  setTimeout(() => {
    const el = document.querySelector(`[data-block-id="${newBlock.id}"]`) as HTMLElement
    el?.focus()
  }, 50)
  emitContent()
}

function serialize() {
  return blocks.value.map(b => b.content).join('\n')
}

function emitContent() {
  emit('update:modelValue', serialize())
}

// Listen for entity clicks and emit entity-click with the entity name
onMounted(() => {
  editorRef.value?.addEventListener('click', (e) => {
    const target = e.target as HTMLElement
    if (target.classList.contains('entity-ref')) {
      const name = target.dataset.entityName
      if (name) emit('entity-click', name)
    }
  })
})
</script>

<style scoped>
.structured-editor {
  display: flex; flex-direction: column; height: 100%;
  font-family: var(--font-serif); font-size: var(--text-md);
  line-height: var(--leading-relaxed);
}
.editor-content {
  flex: 1; overflow-y: auto; padding: var(--space-8) var(--space-16);
}
.editor-block {
  display: flex; gap: var(--space-3); margin-bottom: var(--space-2);
  padding: var(--space-1) 0; border-radius: var(--radius-sm);
  transition: background var(--transition-fast);
}
.editor-block:hover { background: var(--bg-hover); }
.editor-block.active { background: var(--bg-selected); }
.block-gutter {
  width: 24px; display: flex; align-items: flex-start; justify-content: center;
  padding-top: 4px; flex-shrink: 0; opacity: 0.3;
}
.block-type-icon { font-size: 12px; }
.block-content {
  flex: 1; outline: none; min-height: 1.5em; word-break: break-word;
}
.block-content:focus { outline: none; }

/* Block type styles */
.editor-block.dialogue .block-content {
  padding-left: var(--space-4); border-left: 2px solid var(--color-accent);
  color: var(--color-accent);
}
.editor-block.action .block-content {
  color: var(--text-secondary); font-style: italic;
}
.editor-block.narration .block-content {
  color: var(--text-tertiary);
}
.editor-block.heading .block-content {
  font-size: var(--text-lg); font-weight: 600; color: var(--text-primary);
}

/* Entity reference highlighting */
.block-content :deep(.entity-ref) {
  cursor: pointer; border-bottom: 1px dashed var(--color-accent);
  color: var(--color-accent); transition: all var(--transition-fast);
}
.block-content :deep(.entity-ref:hover) { opacity: 0.8; }

.editor-footer {
  display: flex; gap: var(--space-2); padding: var(--space-2) var(--space-4);
  border-top: 1px solid var(--border-muted); flex-shrink: 0;
}
.add-block-btn {
  padding: var(--space-1) var(--space-3); border: 1px solid var(--border-muted);
  background: transparent; color: var(--text-tertiary); border-radius: var(--radius-sm);
  font-size: var(--text-xs); cursor: pointer; font-family: inherit;
  transition: all var(--transition-fast);
}
.add-block-btn:hover { background: var(--bg-hover); color: var(--text-primary); border-color: var(--border-default); }
</style>
