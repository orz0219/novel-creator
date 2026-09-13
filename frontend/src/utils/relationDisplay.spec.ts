import { describe, it, expect } from 'vitest'
import { relationLabel, entityTypeLabel, entityTypeColor } from './relationDisplay'

describe('relationLabel', () => {
  it('把历史数据里的英文 / 蛇形关系翻成中文', () => {
    expect(relationLabel('enemy')).toBe('敌对')
    expect(relationLabel('Enemy')).toBe('敌对')
    expect(relationLabel('CONTROLS')).toBe('控制')
    expect(relationLabel('contains')).toBe('包含')
    expect(relationLabel('LocatedAt')).toBe('位于')
    expect(relationLabel('took_life')).toBe('夺取生命')
    expect(relationLabel('first_believer')).toBe('首位信徒')
  })

  it('未知类型保留原文，不强行猜测', () => {
    expect(relationLabel('custom_relation')).toBe('custom_relation')
  })

  it('空值有兜底', () => {
    expect(relationLabel('')).toBe('关系')
    expect(relationLabel(null)).toBe('关系')
  })
})

describe('entityTypeLabel / entityTypeColor', () => {
  it('实体类型有中文名与稳定颜色', () => {
    expect(entityTypeLabel('Character')).toBe('人物')
    expect(entityTypeLabel('Item')).toBe('物品')
    expect(entityTypeColor('Character')).toBe('#3B82F6')
    expect(entityTypeColor('UnknownType')).toMatch(/^#[0-9A-F]{6}$/i)
  })
})
