//! 「正在生成」前台服务（Android）。
//!
//! 解决的问题：Android 把退到后台的应用进程冻结（cached 状态），
//! App 内的引擎跟着停摆 —— AI 生成跑到一半就断了。
//! 屏幕常亮只能防息屏，防不住用户切到别的应用（实测过）。
//!
//! 前台服务（带常驻通知）让进程保持 active。前端在生成开始时调
//! `set_keep_alive(true)`、结束或失败时调 `false`。
//!
//! 用 Tauri 官方的 Android 插件机制桥接 Kotlin：
//! 之前的 JNI 尝试因为 `ndk_context` 在本进程拿不到 Android context，
//! 直接 panic 把 App 带崩（SIGABRT），所以这次不再走 JNI。

use serde_json::json;
use tauri::plugin::{Builder, PluginHandle, TauriPlugin};
use tauri::Manager;

/// 保存 Kotlin 插件句柄，供命令调用。
///
/// 固定用 `tauri::Wry`：Android 上的 runtime 就是它，
/// 泛型化反而会让 `app.manage` 与命令里的具体类型对不上。
pub struct KeepAlive(PluginHandle<tauri::Wry>);

/// 开关前台服务。
///
/// 失败时把原因原样返回（例如系统拒绝了前台服务），不静默吞掉——
/// 生成能不能在后台继续，用户需要知道。
#[tauri::command]
pub fn set_keep_alive(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        let state = app.state::<KeepAlive>();
        return state
            .0
            .run_mobile_plugin::<serde_json::Value>("setEnabled", json!({ "enabled": enabled }))
            .map(|_| ())
            .map_err(|e| format!("前台服务调用失败: {}", e));
    }
    #[cfg(not(target_os = "android"))]
    {
        // 桌面端没有这个概念：生成本来就跑在后台
        let _ = (app, enabled);
        Ok(())
    }
}

/// 注册插件（Android 上把 Kotlin 侧的 KeepAlivePlugin 接进来）
pub fn init() -> TauriPlugin<tauri::Wry> {
    Builder::new("keepalive")
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            {
                let handle =
                    api.register_android_plugin("com.wangxingchao.novel", "KeepAlivePlugin")?;
                app.manage(KeepAlive(handle));
            }
            #[cfg(not(target_os = "android"))]
            {
                let _ = (app, api);
            }
            Ok(())
        })
        .build()
}
