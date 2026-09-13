// 与 Tauri 原生侧通信的最小封装（不引入打包器）
//
// 说明：手机端页面是原生 JS、无构建步骤，因此直接使用 Tauri 注入的 IPC。

/** 原生侧是否可用（在电脑浏览器里打开时会为 false） */
export const isTauri = typeof window !== 'undefined' && typeof window.__TAURI_INTERNALS__ !== 'undefined'

/** 调用原生命令 */
export async function invoke(cmd, args = {}) {
  if (!isTauri) throw new Error('当前不在手机 App 内运行')
  return window.__TAURI_INTERNALS__.invoke(cmd, args)
}

/**
 * 选择并读取电脑端导出的项目 JSON。
 * 返回 [{ file, content }]；用户取消时返回空数组。
 */
export async function pickImportFiles() {
  return invoke('import_project_file')
}

/**
 * 读取「数据库已重建」的通知（读一次即消费）。
 * 返回提示文案或 null。
 */
export async function takeSchemaRebuildNotice() {
  if (!isTauri) return null
  try {
    return await invoke('take_schema_rebuild_notice')
  } catch (e) {
    console.warn('[tauri] 读取重建通知失败', e)
    return null
  }
}

/** 把诊断信息写进原生日志（App 里没有控制台） */
export async function reportDiagnostic(tag, detail) {
  if (!isTauri) return
  try {
    await invoke('report_diagnostic', { tag, detail })
  } catch (e) {
    console.warn('[tauri] 上报诊断失败', e)
  }
}

/**
 * 把内容导出成手机上的一个文件，返回完整路径。
 * 失败时抛出错误（由调用方展示给用户，不静默）。
 */
export async function exportProjectFile(fileName, content) {
  return invoke('export_project_file', { fileName, content })
}

/** 备份整个数据库，返回备份文件的完整路径 */
export async function backupDatabase() {
  return invoke('backup_database')
}

/**
 * 开关「正在生成」前台服务。
 *
 * Android 会把退到后台的进程冻结，App 里的引擎跟着停摆、生成就断了。
 * 生成开始时打开它（系统通知栏会出现常驻通知），结束/失败时关掉。
 * 桌面端没有这个概念，调用直接返回成功。
 */
export async function setKeepAlive(enabled) {
  if (!isTauri) return
  return invoke('set_keep_alive', { enabled })
}
