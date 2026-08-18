import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useUiStore = defineStore('ui', () => {
  const sidebarCollapsed = ref(false)
  const rightPanelOpen = ref(true)
  const rightPanelWidth = ref(320)
  const leftPanelWidth = ref(240)
  const commandPaletteOpen = ref(false)
  const activeDrawer = ref<string | null>(null)
  const toasts = ref<Toast[]>([])

  interface Toast {
    id: string
    type: 'success' | 'error' | 'warning' | 'info'
    title: string
    message?: string
    duration?: number
  }

  function toggleSidebar() {
    sidebarCollapsed.value = !sidebarCollapsed.value
  }

  function toggleRightPanel() {
    rightPanelOpen.value = !rightPanelOpen.value
  }

  function openCommandPalette() {
    commandPaletteOpen.value = true
  }

  function closeCommandPalette() {
    commandPaletteOpen.value = false
  }

  function addToast(toast: Omit<Toast, 'id'>) {
    const id = Date.now().toString()
    toasts.value.push({ ...toast, id })
    setTimeout(() => removeToast(id), toast.duration || 3000)
  }

  function removeToast(id: string) {
    toasts.value = toasts.value.filter(t => t.id !== id)
  }

  return {
    sidebarCollapsed, rightPanelOpen, rightPanelWidth, leftPanelWidth,
    commandPaletteOpen, activeDrawer, toasts,
    toggleSidebar, toggleRightPanel, openCommandPalette, closeCommandPalette,
    addToast, removeToast,
  }
})
