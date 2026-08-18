import { computed } from 'vue'
import { useRoute } from 'vue-router'
import { useProjectStore } from '@/stores/project'

export function useProject() {
  const route = useRoute()
  const projectStore = useProjectStore()

  const projectId = computed(() => route.params.id as string)
  const currentProject = computed(() => projectStore.currentProject)
  const isLoading = computed(() => projectStore.loading)

  async function loadProject(id: string) {
    await projectStore.fetchProject(id)
  }

  return { projectId, currentProject, isLoading, loadProject }
}
