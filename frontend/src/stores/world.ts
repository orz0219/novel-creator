import { defineStore } from "pinia"
import { ref } from "vue"
import type { Entity, World, Relation, Event, Fact } from "@/types"
import { worldApi, entityApi, relationApi, eventApi, factApi } from "@/api/world"

export const useWorldStore = defineStore("world", () => {
  const currentWorld = ref<World | null>(null)
  const entities = ref<Entity[]>([])
  const relations = ref<Relation[]>([])
  const events = ref<Event[]>([])
  const facts = ref<Fact[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)
  const selectedEntityId = ref<string | null>(null)

  const characters = ref<Entity[]>([])
  const locations = ref<Entity[]>([])
  const factions = ref<Entity[]>([])

  // Fetch world for a project
  async function fetchWorld(projectId: string) {
    loading.value = true
    error.value = null
    try {
      currentWorld.value = await worldApi.get(projectId)
    } catch (e: any) {
      error.value = e.message
    } finally {
      loading.value = false
    }
  }

  // Fetch characters
  async function fetchCharacters(worldId: string) {
    try {
      characters.value = await worldApi.listEntities(worldId, "Character")
    } catch (e: any) {
      error.value = e.message
      characters.value = []
    }
  }

  // Fetch locations
  async function fetchLocations(worldId: string) {
    try {
      locations.value = await worldApi.listEntities(worldId, "Location")
    } catch (e: any) {
      error.value = e.message
      locations.value = []
    }
  }

  // Fetch factions
  async function fetchFactions(worldId: string) {
    try {
      factions.value = await worldApi.listEntities(worldId, "Faction")
    } catch (e: any) {
      error.value = e.message
      factions.value = []
    }
  }

  // Fetch all entities of a type (used for items)
  async function fetchEntities(worldId: string, type?: string): Promise<Entity[]> {
    try {
      return await worldApi.listEntities(worldId, type)
    } catch (e: any) {
      error.value = e.message
      return []
    }
  }

  // Fetch relations
  async function fetchRelations(worldId: string) {
    try {
      relations.value = await relationApi.list(worldId)
    } catch (e: any) {
      error.value = e.message
      relations.value = []
    }
  }

  // Fetch events
  async function fetchEvents(projectId: string) {
    try {
      events.value = await eventApi.list(projectId)
    } catch (e: any) {
      error.value = e.message
      events.value = []
    }
  }

  // Fetch facts
  async function fetchFacts(projectId: string) {
    try {
      facts.value = await factApi.list(projectId)
    } catch (e: any) {
      error.value = e.message
      facts.value = []
    }
  }

  // CRUD: Characters
  async function createCharacter(worldId: string, data: { name: string; summary?: string; description?: string }) {
    const result = await entityApi.createCharacter(worldId, data)
    characters.value.push(result)
    return result
  }

  async function updateCharacter(id: string, data: { name: string; summary?: string; description?: string }) {
    const result = await entityApi.update(id, data)
    const idx = characters.value.findIndex(e => e.id === id)
    if (idx !== -1) characters.value[idx] = result
    return result
  }

  async function deleteCharacter(id: string) {
    await entityApi.delete(id)
    characters.value = characters.value.filter(e => e.id !== id)
  }

  // CRUD: Locations
  async function createLocation(worldId: string, data: { name: string; summary?: string; description?: string }) {
    const result = await entityApi.createLocation(worldId, data)
    locations.value.push(result)
    return result
  }

  async function updateLocation(id: string, data: { name: string; summary?: string; description?: string }) {
    const result = await entityApi.update(id, data)
    const idx = locations.value.findIndex(e => e.id === id)
    if (idx !== -1) locations.value[idx] = result
    return result
  }

  async function deleteLocation(id: string) {
    await entityApi.delete(id)
    locations.value = locations.value.filter(e => e.id !== id)
  }

  // CRUD: Factions
  async function createFaction(worldId: string, data: { name: string; summary?: string; description?: string }) {
    const result = await entityApi.createFaction(worldId, data)
    factions.value.push(result)
    return result
  }

  async function updateFaction(id: string, data: { name: string; summary?: string; description?: string }) {
    const result = await entityApi.update(id, data)
    const idx = factions.value.findIndex(e => e.id === id)
    if (idx !== -1) factions.value[idx] = result
    return result
  }

  async function deleteFaction(id: string) {
    await entityApi.delete(id)
    factions.value = factions.value.filter(e => e.id !== id)
  }

  // CRUD: Relations
  async function createRelation(worldId: string, data: any) {
    const result = await relationApi.create(worldId, data)
    relations.value.push(result)
    return result
  }

  async function deleteRelation(id: string) {
    await relationApi.delete(id)
    relations.value = relations.value.filter(r => r.id !== id)
  }

  // CRUD: Generic entity (items etc)
  async function createEntity(worldId: string, data: { name: string; summary?: string; description?: string }) {
    return await entityApi.create(worldId, data)
  }

  async function deleteEntity(id: string) {
    await entityApi.delete(id)
  }

  function selectEntity(id: string | null) {
    selectedEntityId.value = id
  }

  return {
    currentWorld, entities, relations, events, facts, loading, error,
    selectedEntityId, characters, locations, factions,
    fetchWorld, fetchCharacters, fetchLocations, fetchFactions, fetchEntities,
    fetchRelations, fetchEvents, fetchFacts,
    createCharacter, updateCharacter, deleteCharacter,
    createLocation, updateLocation, deleteLocation,
    createFaction, updateFaction, deleteFaction,
    createRelation, deleteRelation,
    createEntity, deleteEntity,
    selectEntity,
  }
})
