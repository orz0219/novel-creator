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
import { ref, onMounted, watch } from 'vue'

interface EditorBlock {
  id: string
  type: 'paragraph' | 'dialogue' | 'action' | 'narration' | 'heading'
  content: string
  html: string
}

const props = defineProps<{
  modelValue: string
}>()

const emit = defineEmits(['update:modelValue'])

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

// Initialize blocks from modelValue
onMounted(() => {
  if (props.modelValue) {
    const lines = props.modelValue.split('\n')
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
})

function detectBlockType(text: string): EditorBlock['type'] {
  const trimmed = text.trim()
  if (trimmed.startsWith('「') || trimmed.startsWith('"') || trimmed.startsWith('"')) return 'dialogue'
  if (trimmed.startsWith('【') || trimmed.startsWith('---')) return 'heading'
  return 'paragraph'
}

function highlightEntities(text: string): string {
  // Highlight known entity names
  const entities = [
    { name: '林凡', type: 'Character', id: 'char-1' },
    { name: '苏晚晴', type: 'Character', id: 'char-2' },
    { name: '王天德', type: 'Character', id: 'char-3' },
    { name: '黑石城', type: 'Location', id: 'loc-1' },
    { name: '地下遗迹', type: 'Location', id: 'loc-2' },
    { name: '古井', type: 'Location', id: 'loc-3' },
    { name: '王家', type: 'Faction', id: 'fac-1' },
    { name: '黑市', type: 'Faction', id: 'fac-2' },
  ]
  let result = text
  for (const entity of entities) {
    result = result.replace(
      new RegExp(entity.name, 'g'),
      '<span class="entity-ref" data-entity-id="' + entity.id + '" data-entity-type="' + entity.type + '">' + entity.name + '</span>'
    )
  }
  return result
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

function emitContent() {
  const content = blocks.value.map(b => b.content).join('\n')
  emit('update:modelValue', content)
}

// Listen for entity clicks
onMounted(() => {
  editorRef.value?.addEventListener('click', (e) => {
    const target = e.target as HTMLElement
    if (target.classList.contains('entity-ref')) {
      const entityId = target.dataset.entityId
      const entityType = target.dataset.entityType
      console.log('Entity clicked:', entityId, entityType)
      // Emit event to open inspector
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
  cursor: pointer; border-bottom: 1px dashed; transition: all var(--transition-fast);
}
.block-content :deep(.entity-ref:hover) { opacity: 0.8; }
.block-content :deep(.entity-ref[data-entity-type="Character"]) {
  border-color: var(--color-accent); color: var(--color-accent);
}
.block-content :deep(.entity-ref[data-entity-type="Location"]) {
  border-color: var(--color-success); color: var(--color-success);
}
.block-content :deep(.entity-ref[data-entity-type="Faction"]) {
  border-color: var(--color-warning); color: var(--color-warning);
}

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
