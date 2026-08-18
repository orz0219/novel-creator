import { ref, onMounted, onUnmounted } from 'vue'

export function useSelection() {
  const selectedText = ref('')
  const selectionRect = ref<DOMRect | null>(null)
  const hasSelection = ref(false)

  function handleSelection() {
    const sel = window.getSelection()
    if (sel && sel.toString().trim()) {
      selectedText.value = sel.toString()
      const range = sel.getRangeAt(0)
      selectionRect.value = range.getBoundingClientRect()
      hasSelection.value = true
    } else {
      selectedText.value = ''
      selectionRect.value = null
      hasSelection.value = false
    }
  }

  function clearSelection() {
    window.getSelection()?.removeAllRanges()
    selectedText.value = ''
    selectionRect.value = null
    hasSelection.value = false
  }

  onMounted(() => document.addEventListener('selectionchange', handleSelection))
  onUnmounted(() => document.removeEventListener('selectionchange', handleSelection))

  return { selectedText, selectionRect, hasSelection, clearSelection }
}
