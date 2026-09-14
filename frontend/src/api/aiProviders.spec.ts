// 供应商预设表的单元测试：锁定「地址反查」这个易错点。
import { describe, expect, it } from 'vitest'
import {
  AI_PROVIDERS,
  findProvider,
  providerIdForBaseUrl,
  switchProviderKey,
} from './aiProviders'

describe('findProvider', () => {
  it('按 id 取到预设', () => {
    expect(findProvider('commandcode')?.baseUrl).toBe(
      'https://api.commandcode.ai/provider/v1',
    )
  })

  it('未知 id 与空值返回 null（调用方据此判为自定义）', () => {
    expect(findProvider('not-exist')).toBeNull()
    expect(findProvider(undefined)).toBeNull()
  })
})

describe('providerIdForBaseUrl', () => {
  it('精确匹配到对应供应商', () => {
    expect(providerIdForBaseUrl('https://opencode.ai/zen/go/v1')).toBe('opencode')
    expect(providerIdForBaseUrl('https://api.commandcode.ai/provider/v1')).toBe(
      'commandcode',
    )
  })

  it('忽略尾部斜杠与首尾空格', () => {
    expect(providerIdForBaseUrl('  https://opencode.ai/zen/go/v1///  ')).toBe(
      'opencode',
    )
  })

  it('非预设地址与空值落到自定义', () => {
    expect(providerIdForBaseUrl('http://localhost:11434/v1')).toBe('custom')
    expect(providerIdForBaseUrl('')).toBe('custom')
    expect(providerIdForBaseUrl(undefined)).toBe('custom')
  })
})

describe('预设表自身约束', () => {
  it('「自定义」的 baseUrl 为空串（表示不覆盖用户手填地址）', () => {
    const custom = AI_PROVIDERS.find((p) => p.id === 'custom')
    expect(custom?.baseUrl).toBe('')
  })

  it('除自定义外的预设都必须给出接口地址', () => {
    for (const p of AI_PROVIDERS) {
      if (p.id === 'custom') continue
      expect(p.baseUrl.startsWith('https://')).toBe(true)
    }
  })
})

describe('switchProviderKey（两个供应商的 key 必须分离）', () => {
  it('切到另一家：当前 key 存回原供应商，换上新供应商自己的 key', () => {
    const r = switchProviderKey({ commandcode: 'cc-key' }, 'opencode', 'commandcode', 'oc-key')
    expect(r.keys).toEqual({ commandcode: 'cc-key', opencode: 'oc-key' })
    expect(r.key).toBe('cc-key')
  })

  it('切回原供应商：能拿回自己原来的 key', () => {
    // 先切到 commandcode，再切回 opencode
    const first = switchProviderKey({}, 'opencode', 'commandcode', 'oc-key')
    expect(first.key).toBe('')
    const second = switchProviderKey(first.keys, 'commandcode', 'opencode', 'cc-key')
    expect(second.key).toBe('oc-key')
    expect(second.keys.commandcode).toBe('cc-key')
  })

  it('新供应商还没填过 key 时，返回空串而不是上家的 key', () => {
    const r = switchProviderKey({}, 'opencode', 'commandcode', 'oc-key')
    expect(r.key).toBe('')
    expect(r.keys.opencode).toBe('oc-key')
  })

  it('清空输入框后再切走，该供应商的条目被删除（不留空 key）', () => {
    const r = switchProviderKey({ opencode: 'oc-key' }, 'opencode', 'commandcode', '   ')
    expect(r.keys).toEqual({})
    expect(r.key).toBe('')
  })

  it('供应商没变时不改动任何东西（页面初始化回显的场景）', () => {
    const r = switchProviderKey({ opencode: 'oc-key' }, 'opencode', 'opencode', 'hand-typed')
    expect(r.keys).toEqual({ opencode: 'oc-key' })
    expect(r.key).toBe('hand-typed')
  })

  it('不改动传入的账本对象（纯函数）', () => {
    const original = { opencode: 'oc-key' }
    switchProviderKey(original, 'opencode', 'commandcode', 'oc-key')
    expect(original).toEqual({ opencode: 'oc-key' })
  })
})
