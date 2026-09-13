import { describe, it, expect } from 'vitest'
import { displayProfileValue, type DisplayField } from './profileDisplay'

const FIELDS: DisplayField[] = [
  {
    key: 'gender',
    options: [
      { value: '', label: '未设置' },
      { value: 'Male', label: '男' },
      { value: 'Female', label: '女' },
      { value: 'Other', label: '其他' },
    ],
  },
  {
    key: 'age_range',
    options: [
      { value: 'YoungAdult', label: '青年' },
      { value: 'Adult', label: '成年' },
    ],
  },
  { key: 'identity' },
]

describe('displayProfileValue', () => {
  // 这是刚修掉的 bug：档案里显示的是 `YoungAdult` / `Male` 而不是「青年」「男」
  it('枚举字段把存储值翻译成中文标签', () => {
    expect(displayProfileValue('gender', 'Male', FIELDS)).toBe('男')
    expect(displayProfileValue('age_range', 'YoungAdult', FIELDS)).toBe('青年')
    expect(displayProfileValue('age_range', 'Adult', FIELDS)).toBe('成年')
  })

  it('枚举值不在选项里时原样显示（不假装成某个标签）', () => {
    expect(displayProfileValue('gender', 'NonBinary', FIELDS)).toBe('NonBinary')
  })

  it('普通文本字段不做映射', () => {
    expect(displayProfileValue('identity', '程序员', FIELDS)).toBe('程序员')
  })

  it('数组按顿号连接（别名）', () => {
    expect(displayProfileValue('aliases', ['王大锤', '财神'], FIELDS)).toBe('王大锤、财神')
    expect(displayProfileValue('aliases', [], FIELDS)).toBe('')
  })

  it('空值返回空串，交给调用方显示「未填写」', () => {
    expect(displayProfileValue('gender', null, FIELDS)).toBe('')
    expect(displayProfileValue('gender', undefined, FIELDS)).toBe('')
    expect(displayProfileValue('gender', '   ', FIELDS)).toBe('')
  })

  it('字段没在定义里时不影响展示', () => {
    expect(displayProfileValue('unknown_key', 'abc', FIELDS)).toBe('abc')
  })

  it('结构体序列化后展示', () => {
    expect(displayProfileValue('social_position', { rank: '破庙野神' }, FIELDS)).toBe(
      '{"rank":"破庙野神"}',
    )
  })
})
