import { defineStore } from "pinia"
import { ref } from "vue"
import type { Entity, World, Relation, Event, Fact } from "@/types"

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

  function loadMockData() {
    characters.value = [
      {
        id: "char-1", project_id: "p1", world_id: "w1", entity_type_id: "Character",
        name: "林凡", summary: "主角，边境散修", description: "一个从边境走出的年轻散修，性格坚韧，内心善良。",
        attributes: { age: "23", gender: "男", identity: "边境散修", cultivation: "炼气三层" },
        version: 5, created_by: "user", updated_by: "user",
        created_at: "2024-01-15T08:00:00Z", updated_at: "2024-03-12T14:30:00Z"
      },
      {
        id: "char-2", project_id: "p1", world_id: "w1", entity_type_id: "Character",
        name: "苏晚晴", summary: "女主，神秘女子", description: "一位来历神秘的女子，似乎隐藏着不为人知的身份。",
        attributes: { age: "21", gender: "女", identity: "游方修士", cultivation: "筑基初期" },
        version: 3, created_by: "user", updated_by: "ai",
        created_at: "2024-01-20T10:00:00Z", updated_at: "2024-03-10T16:00:00Z"
      },
      {
        id: "char-3", project_id: "p1", world_id: "w1", entity_type_id: "Character",
        name: "王天德", summary: "王家家主", description: "黑石城王家家主，老谋深算，表面儒雅实则狠辣。",
        attributes: { age: "58", gender: "男", identity: "王家家主", cultivation: "金丹初期" },
        version: 2, created_by: "user",
        created_at: "2024-01-18T09:00:00Z", updated_at: "2024-02-28T11:00:00Z"
      },
    ]

    locations.value = [
      {
        id: "loc-1", project_id: "p1", world_id: "w1", entity_type_id: "Location",
        name: "黑石城", summary: "天玄大陆北境重镇", description: "一座以黑色城墙闻名的边境大城。",
        attributes: { type: "城市", region: "北境", population: "50万" },
        version: 4, created_by: "user",
        created_at: "2024-01-15T08:30:00Z", updated_at: "2024-03-05T09:00:00Z"
      },
      {
        id: "loc-2", project_id: "p1", world_id: "w1", entity_type_id: "Location",
        name: "地下遗迹", summary: "黑石城下方的神秘遗迹", description: "据传是远古修士留下的遗迹。",
        attributes: { type: "遗迹", region: "黑石城地下", danger_level: "高" },
        version: 2, created_by: "ai", source_generation_id: "gen-1",
        created_at: "2024-02-01T12:00:00Z", updated_at: "2024-02-15T10:00:00Z"
      },
      {
        id: "loc-3", project_id: "p1", world_id: "w1", entity_type_id: "Location",
        name: "古井", summary: "黑市深处的神秘古井", description: "黑市深处一口不知年代的古井。",
        attributes: { type: "特殊地点", region: "黑市", status: "未知" },
        version: 1, created_by: "ai", source_generation_id: "gen-2",
        created_at: "2024-02-20T14:00:00Z", updated_at: "2024-02-20T14:00:00Z"
      },
    ]

    factions.value = [
      {
        id: "fac-1", project_id: "p1", world_id: "w1", entity_type_id: "Faction",
        name: "王家", summary: "黑石城四大家族之首", description: "黑石城势力最大的家族。",
        attributes: { leader: "王天德", territory: "黑石城东区", members: "300+" },
        version: 3, created_by: "user",
        created_at: "2024-01-16T09:00:00Z", updated_at: "2024-03-01T15:00:00Z"
      },
      {
        id: "fac-2", project_id: "p1", world_id: "w1", entity_type_id: "Faction",
        name: "黑市", summary: "地下势力联盟", description: "黑石城地下世界的掌控者。",
        attributes: { leader: "未知", territory: "黑石城地下", members: "未知" },
        version: 2, created_by: "user",
        created_at: "2024-01-25T11:00:00Z", updated_at: "2024-02-10T09:00:00Z"
      },
    ]

    events.value = [
      {
        id: "evt-1", project_id: "p1", name: "黑石城大火", description: "黑石城东区发生大火，疑为人为纵火。",
        event_type: "灾难", timestamp: "天玄历381年3月10日", event_time: "381-03-10",
        involved_entity_ids: ["loc-1", "fac-1"], state_changes: [],
        created_at: "2024-02-10T08:00:00Z", updated_at: "2024-02-10T08:00:00Z"
      },
      {
        id: "evt-2", project_id: "p1", name: "林凡获得神秘令牌", description: "林凡在古井旁发现一枚黑色令牌。",
        event_type: "发现", timestamp: "天玄历381年3月12日", event_time: "381-03-12",
        involved_entity_ids: ["char-1", "loc-3"], state_changes: [],
        created_at: "2024-02-20T14:30:00Z", updated_at: "2024-02-20T14:30:00Z"
      },
    ]

    facts.value = [
      {
        id: "fact-1", project_id: "p1",
        content: "天玄大陆有三大帝国，分别为天玄帝国、幽冥帝国、龙族帝国。",
        category: "世界格局", certainty: "Confirmed",
        created_at: "2024-01-15T08:00:00Z", updated_at: "2024-01-15T08:00:00Z"
      },
      {
        id: "fact-2", project_id: "p1",
        content: "黑石城是天玄帝国北境最大的城市，以出产黑曜石闻名。",
        category: "地理", certainty: "Confirmed",
        created_at: "2024-01-15T08:00:00Z", updated_at: "2024-01-15T08:00:00Z"
      },
      {
        id: "fact-3", project_id: "p1",
        content: "王家正在秘密追杀林凡，目的是获取他手中的古玉。",
        category: "剧情", certainty: "Likely",
        created_at: "2024-02-15T10:00:00Z", updated_at: "2024-02-15T10:00:00Z"
      },
    ]
  }

  function selectEntity(id: string | null) {
    selectedEntityId.value = id
  }

  return {
    currentWorld, entities, relations, events, facts, loading, error,
    selectedEntityId, characters, locations, factions,
    loadMockData, selectEntity,
  }
})