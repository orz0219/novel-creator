// AI 供应商预设表（单一真源）。
//
// 为什么要有这张表：各家 OpenAI 兼容网关的 base_url 路径**不统一**
// （有的带 `/v1`，有的是 `/provider/v1`），光有供应商名字推不出地址。
// 因此这里把「供应商 → 接口地址」写成静态表，用户选供应商即自动带入地址，
// 只需要再填 API Key。
//
// 约束：**不做任何猜测与兜底**。表里没有的供应商一律选「自定义」自行填地址，
// 不允许按名字模糊猜地址（猜错会得到一个难以排查的连接失败）。

/** 供应商 id（存进 app_settings.aiProvider，仅用于回显下拉选中项）。 */
export type AiProviderId = 'opencode' | 'commandcode' | 'custom'

export interface AiProviderPreset {
  id: AiProviderId
  /** 下拉里显示的名字。 */
  label: string
  /**
   * OpenAI 兼容接口地址前缀（不含 /chat/completions）。
   * 「自定义」为空串：此时不覆盖用户手填的地址。
   */
  baseUrl: string
  /** 选中后显示的补充说明。 */
  hint: string
}

/** 可选供应商（顺序即下拉顺序）。 */
export const AI_PROVIDERS: AiProviderPreset[] = [
  {
    id: 'opencode',
    label: 'opencode.ai',
    baseUrl: 'https://opencode.ai/zen/go/v1',
    hint: 'opencode.ai「Console Go」网关，需额外 x-opencode-session 头（后端已自动携带）。',
  },
  {
    id: 'commandcode',
    label: 'Command Code',
    baseUrl: 'https://api.commandcode.ai/provider/v1',
    hint: 'Command Code 订阅，Bearer token 鉴权，模型名为 vendor/model 形式（如 xiaomi/mimo-v2.5）。',
  },
  {
    id: 'custom',
    label: '自定义',
    baseUrl: '',
    hint: '自建或中转网关：请自行填写接口地址，不会自动覆盖。',
  },
]

/** 按 id 取预设；id 缺失或不在表中返回 null（调用方据此判为「自定义」）。 */
export function findProvider(id: string | undefined): AiProviderPreset | null {
  if (!id) return null
  return AI_PROVIDERS.find((p) => p.id === id) ?? null
}

/** 统一去掉尾部斜杠，用于地址比对。 */
function normalizeUrl(url: string): string {
  return url.trim().replace(/\/+$/, '')
}

/**
 * 按已保存的接口地址反查供应商 id。
 *
 * 用于老数据（库里只有 aiBaseUrl、没有 aiProvider）的回显：地址与某个预设
 * 一致时显示该供应商，否则显示「自定义」。
 */
export function providerIdForBaseUrl(baseUrl: string | undefined): AiProviderId {  const target = normalizeUrl(baseUrl ?? '')
  if (!target) return 'custom'
  const hit = AI_PROVIDERS.find(
    (p) => p.baseUrl !== '' && normalizeUrl(p.baseUrl) === target,
  )
  return hit ? hit.id : 'custom'
}

/**
 * 切换供应商时的密钥账本运算（纯函数，不改入参）。
 *
 * 背景：后端每次调用只读**当前生效**的 `aiApiKey`，因此各供应商的 key 必须
 * 另存一份（`aiApiKeys`）。切换顺序固定为「先备份再替换」：
 * 1. 把输入框里的 key 存回 `previousId` 名下（空则删除该条目）；
 * 2. 取出 `nextId` 自己的 key 作为新的输入框内容。
 *
 * 这样两个 key 完全分离：切到 B 不会污染 A 的 key，切回 A 还能拿回来。
 */
export function switchProviderKey(
  keys: Record<string, string> | undefined,
  previousId: string | undefined,
  nextId: string,
  currentKey: string | undefined,
): { keys: Record<string, string>; key: string } {
  const next = { ...(keys ?? {}) }
  const trimmed = (currentKey ?? '').trim()

  // 供应商没变（页面初始化回显）：保持输入框原值不动
  if (!previousId || previousId === nextId) {
    return { keys: next, key: trimmed }
  }

  if (trimmed) {
    next[previousId] = trimmed
  } else {
    delete next[previousId]
  }
  return { keys: next, key: next[nextId] ?? '' }
}
