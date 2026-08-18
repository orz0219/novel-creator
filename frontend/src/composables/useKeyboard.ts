import { onMounted, onUnmounted } from 'vue'

type KeyHandler = (e: KeyboardEvent) => void

export function useKeyboard(handlers: Record<string, KeyHandler>) {
  function handleKeydown(e: KeyboardEvent) {
    const key = [
      e.metaKey || e.ctrlKey ? 'mod' : '',
      e.shiftKey ? 'shift' : '',
      e.altKey ? 'alt' : '',
      e.key.toLowerCase(),
    ].filter(Boolean).join('+')

    if (handlers[key]) {
      e.preventDefault()
      handlers[key](e)
    }
  }

  onMounted(() => document.addEventListener('keydown', handleKeydown))
  onUnmounted(() => document.removeEventListener('keydown', handleKeydown))
}
