import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { Project, CreateProjectInput, UpdateProjectInput } from '@/types'
import { projectApi } from '@/api/project'

export const useProjectStore = defineStore('project', () => {
  const projects = ref<Project[]>([])
  const currentProject = ref<Project | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  const recentProjects = computed(() =>
    [...projects.value]
      .sort((a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime())
      .slice(0, 5)
  )

  async function fetchProjects() {
    loading.value = true
    error.value = null
    try {
      projects.value = await projectApi.list()
    } catch (e: any) {
      error.value = e.message
    } finally {
      loading.value = false
    }
  }

  async function fetchProject(id: string) {
    loading.value = true
    error.value = null
    try {
      currentProject.value = await projectApi.get(id)
    } catch (e: any) {
      error.value = e.message
    } finally {
      loading.value = false
    }
  }

  async function createProject(input: CreateProjectInput) {
    loading.value = true
    error.value = null
    try {
      const project = await projectApi.create(input)
      projects.value.push(project)
      return project
    } catch (e: any) {
      error.value = e.message
      throw e
    } finally {
      loading.value = false
    }
  }

  async function updateProject(id: string, input: UpdateProjectInput) {
    loading.value = true
    error.value = null
    try {
      const project = await projectApi.update(id, input)
      const index = projects.value.findIndex(p => p.id === id)
      if (index !== -1) projects.value[index] = project
      if (currentProject.value?.id === id) currentProject.value = project
      return project
    } catch (e: any) {
      error.value = e.message
      throw e
    } finally {
      loading.value = false
    }
  }

  async function deleteProject(id: string) {
    loading.value = true
    error.value = null
    try {
      await projectApi.delete(id)
      projects.value = projects.value.filter(p => p.id !== id)
      if (currentProject.value?.id === id) currentProject.value = null
    } catch (e: any) {
      error.value = e.message
      throw e
    } finally {
      loading.value = false
    }
  }

  return {
    projects, currentProject, loading, error, recentProjects,
    fetchProjects, fetchProject, createProject, updateProject, deleteProject,
  }
})
