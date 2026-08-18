<template>
  <Teleport to="body">
    <Transition name="actions">
      <div
        v-if="hasSelection && selectedText"
        class="selection-actions"
        :style="actionStyle"
      >
        <button v-for="action in actions" :key="action.id" class="action-btn" @click="executeAction(action)">
          <span class="action-icon">{{ action.icon }}</span>
          <span class="action-label">{{ action.label }}</span>
        </button>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useSelection } from '@/composables/useSelection'
import { useGeneration } from '@/composables/useGeneration'

const emit = defineEmits(['action'])
const { selectedText, selectionRect, hasSelection } = useSelection()
const { startGeneration } = useGeneration()

const actions = [
  { id: 'rewrite', icon: '✏️', label: '重写', type: 'RewriteSelection' },
  { id: 'expand', icon: '📝', label: '扩展', type: 'ExpandParagraph' },
  { id: 'shorten', icon: '📐', label: '精简', type: 'RewriteSelection' },
  { id: 'continue', icon: '▶️', label: '续写', type: 'GenerateScene' },
  { id: 'tone', icon: '🎭', label: '改风格', type: 'RewriteSelection' },
  { id: 'analyze', icon: '🔍', label: '分析', type: 'AnalyzeCharacter' },
]

const actionStyle = computed(() => {
  if (!selectionRect.value) return {}
  return {
    top: selectionRect.value.top - 40 + 'px',
    left: selectionRect.value.left + selectionRect.value.width / 2 + 'px',
    transform: 'translateX(-50%)',
  }
})

function executeAction(action: any) {
  startGeneration(action.type)
  emit('action', { action: action.id, text: selectedText.value })
}
</script>

<style scoped>
.selection-actions {
  position: fixed; display: flex; gap: var(--space-1);
  background: var(--bg-panel); border: 1px solid var(--border-emphasis);
  border-radius: var(--radius-md); box-shadow: var(--shadow-lg);
  padding: var(--space-1); z-index: var(--z-popover);
}
.action-btn {
  display: flex; align-items: center; gap: var(--space-1);
  padding: var(--space-1) var(--space-2); border: none; background: transparent;
  color: var(--text-secondary); border-radius: var(--radius-sm); cursor: pointer;
  font-size: var(--text-xs); font-family: inherit; transition: all var(--transition-fast);
}
.action-btn:hover { background: var(--bg-hover); color: var(--text-primary); }
.action-icon { font-size: 12px; }
.actions-enter-active, .actions-leave-active { transition: all var(--transition-fast); }
.actions-enter-from, .actions-leave-to { opacity: 0; transform: translateX(-50%) translateY(4px); }
</style>
