/**
 * 人物档案的「表单模型 ↔ 后端契约」映射。
 *
 * 这层之所以必须存在，是因为后端有三类字段不能直接绑到文本框上：
 *
 * | 字段 | 后端形态 | 之前的问题 |
 * |---|---|---|
 * | `age_range` | 枚举 | 前端绑到了一个后端不存在的键 `age`，年龄永远存不进去 |
 * | `gender` | 枚举 | 用自由文本，传「男」被静默降级成 `Other`（错误数据，不是丢失） |
 * | `aliases` | `string[]` | 用文本框，传字符串后反序列化失败被丢弃 |
 * | `social_position` | 结构体 | 当字符串发，同样被丢弃 |
 *
 * 这里把两边的形状差异集中处理，并保证提交时**不悄悄丢掉用户填过的内容**。
 */

import type { CharacterProfile, SocialPosition } from '@/types/character'

export interface ProfileForm {
  name: string
  aliases: string
  age_range: string
  gender: string
  identity: string
  appearance: string
  background_origin: string
  social_position_rank: string
  core_personality: string
  values: string
  role_in_story: string
}

export const AGE_OPTIONS = [
  { value: '', label: '未设置' },
  { value: 'Child', label: '儿童' },
  { value: 'Teen', label: '少年' },
  { value: 'YoungAdult', label: '青年' },
  { value: 'Adult', label: '成年' },
  { value: 'MiddleAge', label: '中年' },
  { value: 'Elder', label: '老年' },
  { value: 'Unknown', label: '不明' },
]

export const GENDER_OPTIONS = [
  { value: '', label: '未设置' },
  { value: 'Male', label: '男' },
  { value: 'Female', label: '女' },
  { value: 'NonBinary', label: '非二元' },
  { value: 'Unknown', label: '不明' },
  { value: 'Other', label: '其他' },
]

export const ROLE_OPTIONS = [
  { value: '', label: '未设置' },
  { value: 'Protagonist', label: '主角' },
  { value: 'Antagonist', label: '反派' },
  { value: 'Mentor', label: '导师' },
  { value: 'Ally', label: '盟友' },
  { value: 'Rival', label: '对手' },
  { value: 'Catalyst', label: '催化剂（引发剧变者）' },
  { value: 'Victim', label: '受害者' },
  { value: 'Observer', label: '旁观者' },
]

export function emptyProfileForm(): ProfileForm {
  return {
    name: '',
    aliases: '',
    age_range: '',
    gender: '',
    identity: '',
    appearance: '',
    background_origin: '',
    social_position_rank: '',
    core_personality: '',
    values: '',
    role_in_story: '',
  }
}

/** 后端档案 → 可编辑表单。数组用「，」拼接，结构体只取 rank。 */
export function profileToForm(p: CharacterProfile): ProfileForm {
  return {
    name: p.name ?? '',
    aliases: (p.aliases ?? []).join('，'),
    age_range: p.age_range ?? '',
    gender: p.gender ?? '',
    identity: p.identity ?? '',
    appearance: p.appearance ?? '',
    background_origin: p.background_origin ?? '',
    social_position_rank: p.social_position?.rank ?? '',
    core_personality: p.core_personality ?? '',
    values: p.values ?? '',
    role_in_story: p.role_in_story ?? '',
  }
}

/**
 * 表单 → 提交体。
 *
 * @param baseProfile 提交前的原始档案。`social_position` 里只有 `rank` 在表单上，
 *   靠它保留 `authority_level` / `social_access` 等未暴露的子字段不被清掉。
 *
 * 枚举留空时**不下发该字段**，避免把库里已有的值覆盖成空。
 */
export function formToProfile(
  f: ProfileForm,
  baseProfile?: CharacterProfile | null,
): Partial<CharacterProfile> {
  const out: Partial<CharacterProfile> = {
    name: f.name.trim() || undefined,
    aliases: f.aliases
      .split(/[,，]/)
      .map((s) => s.trim())
      .filter(Boolean),
    identity: f.identity.trim() || undefined,
    appearance: f.appearance.trim() || undefined,
    background_origin: f.background_origin.trim() || undefined,
    core_personality: f.core_personality.trim() || undefined,
    values: f.values.trim() || undefined,
  }

  if (f.age_range) out.age_range = f.age_range as CharacterProfile['age_range']
  if (f.gender) out.gender = f.gender as CharacterProfile['gender']
  if (f.role_in_story) out.role_in_story = f.role_in_story as CharacterProfile['role_in_story']

  const rank = f.social_position_rank.trim()
  const base: Record<string, unknown> = { ...(baseProfile?.social_position ?? {}) }
  // 只在用户确实动过这一栏、或库里本来就有内容时才提交，
  // 否则会把后端已有的 social_position 无谓地覆盖成空对象
  if (rank || Object.keys(base).length) {
    base.rank = rank || null
    out.social_position = base as SocialPosition
  }

  return out
}
