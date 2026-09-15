// 全局应用设置 API
import { api } from './client'

export interface AppSettings {
  projectName?: string
  language?: string
  /** 模型名（真源：后端每次 LLM 调用前读取本字段）。未按用途单独配置时的保底。 */
  defaultModel?: string
  /**
   * 按用途分配的模型：`{ "<用途key>": "<模型名>" }`。
   *
   * 用途 key 固定四类（与后端 `GenerationPurpose` 一致）：
   * agent（引导对话 / 细纲）、prose（正文生成）、polish（选区改写）、utility（摘要 / 抽取）。
   * 缺失的用途回落到 `defaultModel`。
   */
  taskModels?: Record<string, string>
  /**
   * 按用途分配的温度：`{ "<用途key>": <0-2 的数> }`。
   *
   * 逻辑类压低（少自由发挥）、文学类抬高（留表达空间）；缺失的用途用后端内置默认
   * （agent 0.4 / prose 0.95 / polish 0.85 / utility 0.2）。
   */
  taskTemperatures?: Record<string, number>
  /**
   * 供应商 id（见 aiProviders.ts）。
   * 后端不读该字段，只用于设置页刷新后回显下拉选中项。
   */
  aiProvider?: string
  /** OpenAI 兼容网关前缀，如 https://opencode.ai/zen/go/v1 。 */
  aiBaseUrl?: string
  /** 网关密钥（真源：后端每次 LLM 调用前读取本字段）。 */
  aiApiKey?: string
  /**
   * 各供应商各自的密钥：`{ "<供应商id>": "<key>" }`。
   *
   * 为什么要单独存：`aiApiKey` 是**当前生效**的那一个（后端只读它），
   * 直接切供应商会把上一个供应商的 key 带过去。这里按供应商 id 各存一份，
   * 切换时从对应条目恢复，两个 key 完全分离。
   * 后端不读该字段。
   */
  aiApiKeys?: Record<string, string>
  /** 会话上下文预算（token），聊天页据此显示「已用 / 上限」并预警。 */
  contextLimit?: number
  /** 按模型的上下文上限覆盖：`{ "<模型id>": <token 数> }`（优先于内置目录）。 */
  contextLimits?: Record<string, number>
  /** 单次请求允许模型输出的最大 token 数（默认 22000）。 */
  maxOutputTokens?: number
  fontSize?: number
  autoSave?: boolean
  writingStyle?: string
  autoValidate?: boolean
}

/** 「测试连接」结果：ok=false 时 error 为失败原因。 */
export interface TestConnectionResult {
  ok: boolean
  model?: string
  latency_ms?: number
  reply?: string
  error?: string
}

/** 测试连接入参：留空则用后端当前生效的配置。 */
export interface TestConnectionInput {
  base_url?: string
  api_key?: string
  model?: string
}

/** 「获取模型列表」结果：ok=false 时 error 为失败原因。 */
export interface ModelListResult {
  ok: boolean
  models?: string[]
  error?: string
}

/** 获取模型列表入参：留空则用后端当前生效的配置。 */
export interface ListModelsInput {
  base_url?: string
  api_key?: string
}

/** 内置模型目录：模型 id → 上下文上限（网关不返回该数据，故由后端内置）。 */
export interface ModelCatalog {
  /** 目录未收录模型时的默认上限。 */
  default_limit: number
  context_limits: Record<string, number>
}

export const settingsApi = {
  get: () => api.get<AppSettings>('/settings'),
  update: (data: AppSettings) => api.put<AppSettings>('/settings', data),
  testConnection: (data: TestConnectionInput) =>
    api.post<TestConnectionResult>('/settings/test-connection', data),
  /** 拉取网关真实可用的模型列表（OpenAI 兼容 GET /models）。 */
  listModels: (data: ListModelsInput) =>
    api.post<ModelListResult>('/settings/models', data),
  /** 拉取内置模型目录（模型 → 上下文上限）。 */
  getModelCatalog: () => api.get<ModelCatalog>('/settings/model-catalog'),
}
