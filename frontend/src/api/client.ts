// API Client - 统一的 HTTP 客户端

const BASE_URL = '/api/v1'

async function request<T>(url: string, options: RequestInit = {}): Promise<T> {
  const response = await fetch(BASE_URL + url, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options.headers,
    },
  })

  if (!response.ok) {
    const body = await response.json().catch(() => null)
    const detail = body?.message || body?.error || response.statusText
    throw new Error(`HTTP ${response.status} ${url} — ${detail}`)
  }

  if (response.status === 204) return undefined as T
  return response.json()
}

export const api = {
  get: <T>(url: string) => request<T>(url),
  post: <T>(url: string, data?: unknown) => request<T>(url, { method: 'POST', body: data ? JSON.stringify(data) : undefined }),
  put: <T>(url: string, data?: unknown) => request<T>(url, { method: 'PUT', body: data ? JSON.stringify(data) : undefined }),
  patch: <T>(url: string, data?: unknown) => request<T>(url, { method: 'PATCH', body: data ? JSON.stringify(data) : undefined }),
  delete: <T>(url: string) => request<T>(url, { method: 'DELETE' }),
}
