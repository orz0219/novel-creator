/**
 * 金手指结构化档案的读取与校验。
 *
 * 金手指的档案存放在 `entity.attributes`（jsonb），结构与后端
 * `update_golden_finger` 工具的写入契约一致。这里**不做兜底填充**：
 *
 * - 一个字段都没填 → `empty`（正常状态：还没聊到这一步）
 * - 填了但结构不符契约 → `invalid` 并逐条列出问题（这是真问题，必须让人看见）
 * - 结构正确 → `ready`
 *
 * 之所以不把非法结构当成空对象静默处理：那会让「AI 写错了字段名」
 * 表现成「面板一片空白」，与「还没填」完全无法区分，问题永远查不出来。
 */

export interface GoldenFingerAbility {
  name: string
  effect: string
  trigger?: string
  limit?: string
}

export interface GoldenFingerStage {
  stage: string
  unlocked: string
  note?: string
}

export interface GoldenFingerProfile {
  gf_type?: string
  one_liner?: string
  origin?: string
  abilities: GoldenFingerAbility[]
  side_effect?: string
  constraints: string[]
  growth_stages: GoldenFingerStage[]
}

export type GoldenFingerRead =
  | { kind: 'empty' }
  | { kind: 'invalid'; problems: string[] }
  | { kind: 'ready'; profile: GoldenFingerProfile }

/** 金手指契约中的全部字段名（与后端工具保持一致）。 */
export const GOLDEN_FINGER_KEYS = [
  'gf_type',
  'one_liner',
  'origin',
  'abilities',
  'side_effect',
  'constraints',
  'growth_stages',
] as const

const SCALAR_KEYS = ['gf_type', 'one_liner', 'origin', 'side_effect'] as const

function readScalar(
  attrs: Record<string, unknown>,
  key: string,
  problems: string[],
): string | undefined {
  const v = attrs[key]
  if (v === undefined || v === null) return undefined
  if (typeof v !== 'string') {
    problems.push(`字段 ${key} 应为字符串，实际是 ${typeof v}`)
    return undefined
  }
  const trimmed = v.trim()
  return trimmed || undefined
}

function readAbilities(attrs: Record<string, unknown>, problems: string[]): GoldenFingerAbility[] {
  const v = attrs.abilities
  if (v === undefined || v === null) return []
  if (!Array.isArray(v)) {
    problems.push('字段 abilities 应为数组')
    return []
  }
  const out: GoldenFingerAbility[] = []
  v.forEach((item, i) => {
    if (typeof item !== 'object' || item === null || Array.isArray(item)) {
      problems.push(`abilities[${i}] 应为对象`)
      return
    }
    const obj = item as Record<string, unknown>
    const name = typeof obj.name === 'string' ? obj.name.trim() : ''
    const effect = typeof obj.effect === 'string' ? obj.effect.trim() : ''
    if (!name) problems.push(`abilities[${i}] 缺少 name`)
    if (!effect) problems.push(`abilities[${i}] 缺少 effect`)
    if (!name || !effect) return
    out.push({
      name,
      effect,
      trigger: typeof obj.trigger === 'string' && obj.trigger.trim() ? obj.trigger.trim() : undefined,
      limit: typeof obj.limit === 'string' && obj.limit.trim() ? obj.limit.trim() : undefined,
    })
  })
  return out
}

function readConstraints(attrs: Record<string, unknown>, problems: string[]): string[] {
  const v = attrs.constraints
  if (v === undefined || v === null) return []
  if (!Array.isArray(v)) {
    problems.push('字段 constraints 应为字符串数组')
    return []
  }
  const out: string[] = []
  v.forEach((item, i) => {
    if (typeof item !== 'string') {
      problems.push(`constraints[${i}] 应为字符串`)
      return
    }
    if (item.trim()) out.push(item.trim())
  })
  return out
}

function readStages(attrs: Record<string, unknown>, problems: string[]): GoldenFingerStage[] {
  const v = attrs.growth_stages
  if (v === undefined || v === null) return []
  if (!Array.isArray(v)) {
    problems.push('字段 growth_stages 应为数组')
    return []
  }
  const out: GoldenFingerStage[] = []
  v.forEach((item, i) => {
    if (typeof item !== 'object' || item === null || Array.isArray(item)) {
      problems.push(`growth_stages[${i}] 应为对象`)
      return
    }
    const obj = item as Record<string, unknown>
    const stage = typeof obj.stage === 'string' ? obj.stage.trim() : ''
    const unlocked = typeof obj.unlocked === 'string' ? obj.unlocked.trim() : ''
    if (!stage) problems.push(`growth_stages[${i}] 缺少 stage`)
    if (!unlocked) problems.push(`growth_stages[${i}] 缺少 unlocked`)
    if (!stage || !unlocked) return
    out.push({
      stage,
      unlocked,
      note: typeof obj.note === 'string' && obj.note.trim() ? obj.note.trim() : undefined,
    })
  })
  return out
}

/**
 * 读取金手指档案。
 *
 * 注意 `attributes` 里可能同时躺着其它用途的键（例如别的功能写的元数据），
 * 因此只有「一个金手指字段都没有」才算 `empty`。
 */
export function readGoldenFinger(attributes: Record<string, unknown> | null | undefined): GoldenFingerRead {
  const attrs = attributes ?? {}
  const present = GOLDEN_FINGER_KEYS.some((k) => attrs[k] !== undefined && attrs[k] !== null)
  if (!present) return { kind: 'empty' }

  const problems: string[] = []
  const scalars: Record<string, string | undefined> = {}
  for (const key of SCALAR_KEYS) {
    scalars[key] = readScalar(attrs, key, problems)
  }

  const profile: GoldenFingerProfile = {
    gf_type: scalars.gf_type,
    one_liner: scalars.one_liner,
    origin: scalars.origin,
    side_effect: scalars.side_effect,
    abilities: readAbilities(attrs, problems),
    constraints: readConstraints(attrs, problems),
    growth_stages: readStages(attrs, problems),
  }

  if (problems.length) return { kind: 'invalid', problems }
  return { kind: 'ready', profile }
}

/** 档案里真正填了多少"有内容"的部分——用于显示完成度。 */
export function profileCompleteness(profile: GoldenFingerProfile): { filled: number; total: number } {
  const checks = [
    !!profile.gf_type,
    !!profile.one_liner,
    !!profile.origin,
    profile.abilities.length > 0,
    !!profile.side_effect,
    profile.constraints.length > 0,
    profile.growth_stages.length > 0,
  ]
  return { filled: checks.filter(Boolean).length, total: checks.length }
}
