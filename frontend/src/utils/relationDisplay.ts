// 关系 / 实体类型的面向用户展示映射。
//
// 数据层允许模型写任意 relation_type（历史数据里混有 enemy、CONTROLS、took_life
// 这类英文/蛇形命名），但 UI 是中文写作工具，展示层必须统一翻成中文。
// 这里只做“展示翻译”，不改数据库真源。

const RELATION_LABELS: Record<string, string> = {
  // 冲突 / 敌对
  enemy: '敌对',
  hostile: '敌对',
  conflict: '冲突',
  conflictwith: '冲突',
  opposing: '对立',
  rival: '竞争',
  // 合作 / 友好
  friend: '朋友',
  ally: '盟友',
  best_friend: '挚友',
  bestfriend: '挚友',
  partner: '伙伴',
  member: '成员',
  memberof: '隶属',
  joined: '加入',
  serves: '效忠',
  leaderof: '领导',
  leader: '领袖',
  founder: '创立者',
  founded: '创立',
  studentof: '师从',
  mentor: '导师',
  teacher: '老师',
  // 控制 / 拥有 / 空间
  controls: '控制',
  control: '控制',
  owns: '拥有',
  possesses: '拥有',
  has: '拥有',
  contains: '包含',
  part_of: '属于',
  partof: '属于',
  locatedat: '位于',
  locatedin: '位于',
  livesin: '居住于',
  near: '邻近',
  landlord_of: '房东',
  landlordof: '房东',
  tenantof: '租客',
  // 亲属 / 情感
  parent: '父母',
  parentof: '父母',
  child: '子女',
  childof: '子女',
  sibling: '兄弟姐妹',
  spouse: '配偶',
  lover: '恋人',
  marriedto: '配偶',
  // 剧情 / 因果
  involves: '涉及',
  involvedin: '参与',
  caused: '导致',
  took_life: '夺取生命',
  tooklife: '夺取生命',
  killed: '杀死',
  saved: '救下',
  protects: '保护',
  guardianof: '守护',
  first_believer: '首位信徒',
  believerof: '信仰',
  worships: '信仰',
  // 其他常见
  created: '创建',
  createdby: '由…创建',
  uses: '使用',
  usedby: '被…使用',
  knows: '认识',
  friendof: '朋友',
}

function normalizeKey(raw: string): string {
  return raw.trim().toLowerCase().replace(/[\s-]+/g, '_')
}

/** 关系类型 → 中文展示名。未知类型原样返回（不强行翻译导致语义错误）。 */
export function relationLabel(raw: string | undefined | null): string {
  const text = (raw ?? '').trim()
  if (!text) return '关系'
  const key = normalizeKey(text)
  return RELATION_LABELS[key] ?? text
}

export interface EntityTypeMeta {
  label: string
  color: string
}

const ENTITY_TYPE_META: Record<string, EntityTypeMeta> = {
  character: { label: '人物', color: '#3B82F6' },
  location: { label: '地点', color: '#10B981' },
  faction: { label: '势力', color: '#F59E0B' },
  item: { label: '物品', color: '#A855F7' },
  creature: { label: '生物', color: '#14B8A6' },
  organization: { label: '组织', color: '#EAB308' },
  event: { label: '事件', color: '#EC4899' },
  golden_finger: { label: '金手指', color: '#C84B31' },
  race: { label: '种族', color: '#84CC16' },
  nation: { label: '国家', color: '#F97316' },
  city: { label: '城市', color: '#06B6D4' },
  sect: { label: '宗门', color: '#8B5CF6' },
  deity: { label: '神祇', color: '#F43F5E' },
  concept: { label: '概念', color: '#64748B' },
  technology: { label: '科技', color: '#0EA5E9' },
  thread: { label: '剧情线', color: '#D946EF' },
}

const FALLBACK_COLORS = [
  '#3B82F6', '#10B981', '#F59E0B', '#A855F7', '#EC4899',
  '#14B8A6', '#F97316', '#8B5CF6', '#06B6D4', '#84CC16',
]

function hashString(s: string): number {
  let hash = 0
  for (let i = 0; i < s.length; i++) hash = (hash * 31 + s.charCodeAt(i)) | 0
  return Math.abs(hash)
}

/** 实体类型 → 中文展示名。 */
export function entityTypeLabel(raw: string | undefined | null): string {
  const text = (raw ?? '').trim()
  if (!text) return '实体'
  return ENTITY_TYPE_META[text.toLowerCase()]?.label ?? text
}

/** 实体类型 → 图谱颜色。未知类型用稳定哈希给一个不重复的颜色。 */
export function entityTypeColor(raw: string | undefined | null): string {
  const text = (raw ?? '').trim()
  if (!text) return '#64748B'
  return ENTITY_TYPE_META[text.toLowerCase()]?.color
    ?? FALLBACK_COLORS[hashString(text) % FALLBACK_COLORS.length]
}
