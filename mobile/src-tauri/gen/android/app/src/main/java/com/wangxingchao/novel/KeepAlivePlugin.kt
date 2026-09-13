package com.wangxingchao.novel

import android.app.Activity
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.Plugin

/** `keepalive` 插件的入参 */
@InvokeArg
class SetEnabledArgs {
  var enabled: Boolean = false
}

/**
 * 把「前台服务」暴露给 Rust/前端。
 *
 * Rust 侧通过 `register_android_plugin("com.wangxingchao.novel", "KeepAlivePlugin")`
 * 拿到句柄，再用 `run_mobile_plugin("setEnabled", {enabled})` 调用。
 *
 * 用 Tauri 官方的插件机制而不是 JNI：之前试过 JNI 拿 Android context，
 * `ndk_context` 在本进程取不到，直接把 App 带崩（SIGABRT）。
 */
@TauriPlugin
class KeepAlivePlugin(private val activity: Activity) : Plugin(activity) {
  @Command
  fun setEnabled(invoke: Invoke) {
    val args = invoke.parseArgs(SetEnabledArgs::class.java)
    if (args.enabled) {
      KeepAliveService.start(activity)
    } else {
      KeepAliveService.stop(activity)
    }
    invoke.resolve()
  }
}
