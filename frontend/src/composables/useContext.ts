import { computed } from 'vue'
import { useContextStore } from '@/stores/context'

export function useContext() {
  const contextStore = useContextStore()

  const entities = computed(() => contextStore.entities)
  const items = computed(() => contextStore.items)
  const totalTokens = computed(() => contextStore.totalTokens)

  const pinnedEntities = computed(() =>
    contextStore.entities.filter(e => e.policy === 'Pinned')
  )
  const excludedEntities = computed(() =>
    contextStore.entities.filter(e => e.policy === 'Excluded')
  )
  const autoEntities = computed(() =>
    contextStore.entities.filter(e => e.policy === 'Automatic')
  )

  function togglePin(entityId: string) { contextStore.togglePin(entityId) }
  function toggleExclude(entityId: string) { contextStore.toggleExclude(entityId) }

  return {
    entities, items, totalTokens,
    pinnedEntities, excludedEntities, autoEntities,
    togglePin, toggleExclude,
  }
}
