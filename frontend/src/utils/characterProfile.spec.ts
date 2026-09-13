import { describe, it, expect } from 'vitest'
import {
  profileToForm,
  formToProfile,
  emptyProfileForm,
  type ProfileForm,
} from './characterProfile'
import type { CharacterProfile } from '@/types/character'

describe('profileToForm', () => {
  it('从后端档案读出可编辑值（数组拼接、结构体取 rank）', () => {
    const p: CharacterProfile = {
      id: 'x',
      entity_id: 'y',
      name: '王久财',
      aliases: ['王大锤', '财神'],
      age_range: 'YoungAdult',
      gender: 'Male',
      identity: '程序员',
      social_position: { rank: '破庙野神', authority_level: 3 },
      role_in_story: 'Protagonist',
    } as CharacterProfile
    const f = profileToForm(p)
    expect(f.name).toBe('王久财')
    expect(f.aliases).toBe('王大锤，财神')
    expect(f.age_range).toBe('YoungAdult')
    expect(f.gender).toBe('Male')
    expect(f.social_position_rank).toBe('破庙野神')
    expect(f.role_in_story).toBe('Protagonist')
  })

  it('档案为 null 字段时给出空串，而不是 undefined', () => {
    const f = profileToForm({ id: 'x', entity_id: 'y' } as CharacterProfile)
    expect(f.name).toBe('')
    expect(f.aliases).toBe('')
    expect(f.gender).toBe('')
    expect(f.social_position_rank).toBe('')
  })
})

describe('formToProfile', () => {
  const base = (over: Partial<ProfileForm> = {}): ProfileForm => ({
    ...emptyProfileForm(),
    ...over,
  })

  it('别名按中英文逗号拆分并去空白', () => {
    const out = formToProfile(base({ aliases: '王大锤, 财神，老王' }))
    expect(out.aliases).toEqual(['王大锤', '财神', '老王'])
  })

  it('别名留空时提交空数组（清空），不留下脏数据', () => {
    const out = formToProfile(base({ aliases: '   ' }))
    expect(out.aliases).toEqual([])
  })

  it('枚举按规范值提交 —— 这是修复「填了男却存成 Other」的关键', () => {
    const out = formToProfile(base({ gender: 'Male', age_range: 'YoungAdult', role_in_story: 'Protagonist' }))
    expect(out.gender).toBe('Male')
    expect(out.age_range).toBe('YoungAdult')
    expect(out.role_in_story).toBe('Protagonist')
  })

  it('枚举未选择时不下发该字段，避免把库里已有值覆盖成空', () => {
    const out = formToProfile(base({ name: '王久财' }))
    expect('gender' in out).toBe(false)
    expect('age_range' in out).toBe(false)
    expect('role_in_story' in out).toBe(false)
  })

  it('提交社会地位时保留未在表单上暴露的子字段（authority_level / social_access）', () => {
    const prev = {
      id: 'x',
      entity_id: 'y',
      social_position: { rank: '旧头衔', authority_level: 5, social_access: ['宗门'] },
    } as CharacterProfile
    const out = formToProfile(base({ social_position_rank: '破庙野神' }), prev)
    expect(out.social_position).toEqual({
      rank: '破庙野神',
      authority_level: 5,
      social_access: ['宗门'],
    })
  })

  it('清空社会地位时显式传 null，让后端清掉 rank 而不是整块丢弃', () => {
    const prev = {
      id: 'x',
      entity_id: 'y',
      social_position: { rank: '旧头衔', authority_level: 5 },
    } as CharacterProfile
    const out = formToProfile(base({ social_position_rank: '' }), prev)
    expect(out.social_position).toEqual({ rank: null, authority_level: 5 })
  })

  it('库里本来就没有社会地位、用户也没填时不提交该字段', () => {
    const out = formToProfile(base({ name: '王久财' }), { id: 'x', entity_id: 'y' } as CharacterProfile)
    expect('social_position' in out).toBe(false)
  })

  it('纯文本字段留空时不下发（undefined 表示不改动）', () => {
    const out = formToProfile(base({ name: '王久财', identity: '   ' }))
    expect(out.name).toBe('王久财')
    expect(out.identity).toBeUndefined()
  })
})
