import { describe, it, expect } from 'vitest'
import { readGoldenFinger, profileCompleteness } from './goldenFinger'

describe('readGoldenFinger', () => {
  it('没有金手指字段时判为 empty（还没聊到这一步）', () => {
    expect(readGoldenFinger({}).kind).toBe('empty')
    expect(readGoldenFinger(null).kind).toBe('empty')
    expect(readGoldenFinger(undefined).kind).toBe('empty')
  })

  it('attributes 里只有其它用途的键时仍判为 empty', () => {
    // 别的功能可能往同一个 jsonb 里写元数据，不应被误判成"档案填错了"
    expect(readGoldenFinger({ some_other_meta: 1, version_tag: 'x' }).kind).toBe('empty')
  })

  it('完整结构判为 ready 并原样读出', () => {
    const read = readGoldenFinger({
      gf_type: '神格·信仰系统',
      one_liner: '把信徒的信仰变成钱和神力',
      origin: '攫取神明残留的神格',
      side_effect: '无',
      abilities: [{ name: '信仰收集', effect: '每人每日 5 点', trigger: '有信徒', limit: '只与人数成正比' }],
      constraints: ['领域外神力无用'],
      growth_stages: [{ stage: '初期', unlocked: '攒信仰换钱', note: '神域仅限破庙' }],
    })
    expect(read.kind).toBe('ready')
    if (read.kind !== 'ready') return
    expect(read.profile.gf_type).toBe('神格·信仰系统')
    expect(read.profile.abilities).toHaveLength(1)
    expect(read.profile.abilities[0].limit).toBe('只与人数成正比')
    expect(read.profile.constraints).toEqual(['领域外神力无用'])
    expect(read.profile.growth_stages[0].note).toBe('神域仅限破庙')
  })

  it('abilities 不是数组时判为 invalid，而不是当成空', () => {
    const read = readGoldenFinger({ gf_type: '系统', abilities: '信仰收集' })
    expect(read.kind).toBe('invalid')
    if (read.kind !== 'invalid') return
    expect(read.problems.some((p) => p.includes('abilities'))).toBe(true)
  })

  it('ability 缺 name / effect 时逐条报出来', () => {
    const read = readGoldenFinger({ abilities: [{ effect: '有' }, { name: '有' }] })
    expect(read.kind).toBe('invalid')
    if (read.kind !== 'invalid') return
    expect(read.problems).toContain('abilities[0] 缺少 name')
    expect(read.problems).toContain('abilities[1] 缺少 effect')
  })

  it('constraints 里混入非字符串时报错', () => {
    const read = readGoldenFinger({ constraints: ['ok', 42] })
    expect(read.kind).toBe('invalid')
    if (read.kind !== 'invalid') return
    expect(read.problems).toContain('constraints[1] 应为字符串')
  })

  it('growth_stages 缺 stage / unlocked 时报错', () => {
    const read = readGoldenFinger({ growth_stages: [{ stage: '初期' }, { unlocked: '能打' }] })
    expect(read.kind).toBe('invalid')
    if (read.kind !== 'invalid') return
    expect(read.problems).toContain('growth_stages[0] 缺少 unlocked')
    expect(read.problems).toContain('growth_stages[1] 缺少 stage')
  })

  it('标量字段类型不对时报错', () => {
    const read = readGoldenFinger({ one_liner: 123 })
    expect(read.kind).toBe('invalid')
    if (read.kind !== 'invalid') return
    expect(read.problems).toContain('字段 one_liner 应为字符串，实际是 number')
  })

  it('空串视为未填写，不报错', () => {
    const read = readGoldenFinger({ gf_type: '系统', one_liner: '   ' })
    expect(read.kind).toBe('ready')
    if (read.kind !== 'ready') return
    expect(read.profile.one_liner).toBeUndefined()
  })

  it('列表项里的空白串被清理掉', () => {
    const read = readGoldenFinger({ constraints: [' 有意义的约束 ', '   '] })
    expect(read.kind).toBe('ready')
    if (read.kind !== 'ready') return
    expect(read.profile.constraints).toEqual(['有意义的约束'])
  })
})

describe('profileCompleteness', () => {
  it('全空时是 0/7', () => {
    const c = profileCompleteness({
      abilities: [],
      constraints: [],
      growth_stages: [],
    })
    expect(c).toEqual({ filled: 0, total: 7 })
  })

  it('逐项计入已填字段', () => {
    const c = profileCompleteness({
      gf_type: '系统',
      one_liner: '攒钱',
      origin: '天降',
      side_effect: '无',
      abilities: [{ name: 'a', effect: 'b' }],
      constraints: ['c'],
      growth_stages: [],
    })
    expect(c).toEqual({ filled: 6, total: 7 })
  })
})
