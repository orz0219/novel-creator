import { defineStore } from "pinia"
import { ref } from "vue"
import type { Proposal } from "@/types"
import { proposalApi } from "@/api/proposal"

export const useProposalStore = defineStore("proposal", () => {
  const proposals = ref<Proposal[]>([])
  const currentProposal = ref<Proposal | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchProposals(projectId: string) {
    loading.value = true
    error.value = null
    try {
      proposals.value = await proposalApi.list(projectId)
    } catch (e: any) {
      error.value = e.message
      proposals.value = []
    } finally {
      loading.value = false
    }
  }

  async function fetchProposal(id: string) {
    loading.value = true
    try {
      currentProposal.value = await proposalApi.get(id)
    } catch (e: any) {
      error.value = e.message
    } finally {
      loading.value = false
    }
  }

  async function acceptProposal(id: string) {
    await proposalApi.accept(id)
    const idx = proposals.value.findIndex(p => p.id === id)
    if (idx !== -1) proposals.value[idx].status = "Approved"
  }

  async function rejectProposal(id: string) {
    await proposalApi.reject(id)
    const idx = proposals.value.findIndex(p => p.id === id)
    if (idx !== -1) proposals.value[idx].status = "Rejected"
  }

  async function acceptChange(proposalId: string, changeId: string) {
    await proposalApi.acceptChange(proposalId, changeId)
  }

  async function rejectChange(proposalId: string, changeId: string) {
    await proposalApi.rejectChange(proposalId, changeId)
  }

  return {
    proposals, currentProposal, loading, error,
    fetchProposals, fetchProposal,
    acceptProposal, rejectProposal, acceptChange, rejectChange,
  }
})
