//! Novel Engine 手机单机版 - 应用入口
//!
//! 启动时在进程内拉起完整引擎（SQLite + axum on 127.0.0.1:8080），
//! WebView 通过 http://127.0.0.1:8080/api/v1/* 访问，与电脑端接口完全一致。

use tauri::Manager;

/// 「正在生成」前台服务（Android 上让进程不被系统冻结）
mod keepalive;

/// 日志初始化：Android 走 logcat（标签 novel-mobile）
fn init_logging() {
    #[cfg(target_os = "android")]
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("novel-mobile"),
    );
}

/// 引擎监听端口。只绑定 127.0.0.1，不对外暴露。
const ENGINE_PORT: u16 = 8080;

/// 启动内嵌引擎（异步，不阻塞 UI）
/// 记录应用可用的各个目录，便于确定导出文件该放哪里（用户要能找到）
fn log_paths(app: &tauri::AppHandle) {
    use tauri::Manager;
    let p = app.path();
    log::info!("app_data_dir       = {:?}", p.app_data_dir().ok());
    log::info!("app_local_data_dir = {:?}", p.app_local_data_dir().ok());
    log::info!("app_cache_dir      = {:?}", p.app_cache_dir().ok());
    log::info!("app_config_dir     = {:?}", p.app_config_dir().ok());

    match export_dir(app) {
        Ok(d) => log::info!("export_dir(可写)   = {}", d.display()),
        Err(e) => log::error!("export_dir 不可用: {}", e),
    }
}

fn start_engine(app: &tauri::AppHandle) {
    let data_dir = match app.path().app_data_dir() {
        Ok(d) => d,
        Err(e) => {
            log::error!("无法获取应用数据目录: {}", e);
            return;
        }
    };

    if let Err(e) = std::fs::create_dir_all(&data_dir) {
        log::error!("创建数据目录失败: {}", e);
        return;
    }

    let db_path = data_dir.join("novel.db");
    log::info!("数据库位置: {}", db_path.display());

    // 首次启动用的默认 AI 配置（构建时从仓库 .env 注入；用户在设置页改过则以其为准）
    let bootstrap = sqlite_engine::engine::BootstrapConfig {
        api_key: env!("NOVEL_BOOT_API_KEY").to_string(),
        base_url: env!("NOVEL_BOOT_BASE_URL").to_string(),
        model: env!("NOVEL_BOOT_MODEL").to_string(),
        max_output_tokens: env!("NOVEL_BOOT_MAX_OUTPUT_TOKENS")
            .parse()
            .expect("NOVEL_BOOT_MAX_OUTPUT_TOKENS 必须是正整数"),
    };

    // 用独立线程 + 独立 tokio runtime 承载引擎。
    //
    // 原因：tauri::async_runtime 的 spawn 在 Android 目标上会对 sqlx 的
    // 连接操作触发 "implementation of Send is not general enough" 的
    // 生命周期约束冲突（sqlx 的 &mut SqliteConnection 执行器带 '0 生命周期）。
    // 独立线程里跑自己的 runtime 可以完全绕开这个约束。
    std::thread::Builder::new()
        .name("novel-engine".into())
        .spawn(move || {
            let rt = match tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    log::error!("创建引擎运行时失败: {}", e);
                    return;
                }
            };

            rt.block_on(async move {
                let db = db_path.to_string_lossy().to_string();
                match sqlite_engine::engine::Engine::build(&db, bootstrap).await {
                    Ok(engine) => {
                        log::info!("引擎组装完成，开始监听 127.0.0.1:{}", ENGINE_PORT);
                        if let Err(e) = engine.serve(ENGINE_PORT).await {
                            log::error!("引擎服务退出: {}", e);
                        }
                    }
                    Err(e) => {
                        log::error!("引擎组装失败: {:#}", e);
                    }
                }
            });
        })
        .expect("无法启动引擎线程");
}

/// 让用户选择电脑端导出的项目 JSON，读进来逐个导入。
///
/// 为什么把「选文件 + 读文件」都放在 Rust 侧：
///   Android 上 WebView 不支持 `<input type="file">`，必须用系统文件选择器；
///   而选择器返回的是 content:// URI，前端再去读会受作用域存储限制。
///   这里用 tauri-plugin-fs 在原生侧读取，绕开该限制。
#[tauri::command]
async fn import_project_file(app: tauri::AppHandle) -> Result<Vec<ImportOutcome>, String> {
    use tauri_plugin_dialog::DialogExt;

    let picked = app
        .dialog()
        .file()
        .add_filter("项目导出 JSON", &["json"])
        .blocking_pick_files();

    let Some(paths) = picked else {
        return Ok(Vec::new()); // 用户取消
    };

    let mut out = Vec::new();
    for p in paths {
        let file_path = p
            .into_path()
            .map_err(|e| format!("无法解析所选文件路径: {}", e))?;
        let name = file_path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "未命名文件".into());

        let text = tokio::fs::read_to_string(&file_path)
            .await
            .map_err(|e| format!("读取 {} 失败: {}", name, e))?;

        out.push(ImportOutcome {
            file: name,
            content: text,
        });
    }
    Ok(out)
}

/// 读取并消费「数据库已重建」通知。
///
/// 引擎在结构代号不匹配时会重建数据库并写出 `schema_rebuilt` 标记，
/// 这里把它读出来交给前端提示用户（读一次即删除，避免每次启动都提示）。
#[tauri::command]
fn take_schema_rebuild_notice(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("无法获取应用数据目录: {}", e))?;
    let marker = dir.join("schema_rebuilt");
    if !marker.exists() {
        return Ok(None);
    }
    let gen = std::fs::read_to_string(&marker).unwrap_or_else(|_| "?".into());
    if let Err(e) = std::fs::remove_file(&marker) {
        log::warn!("删除重建标记失败: {}", e);
    }
    log::info!("检测到数据库已因结构变更重建（代号 {}）", gen.trim());
    Ok(Some(format!(
        "数据库结构已更新，手机上的数据已重置（代号 {}）",
        gen.trim()
    )))
}

/// 磁盘上的导出目录（手机端专用）。
///
/// 优先写到**外部专属目录** `Android/data/<pkg>/files/export`：
/// 该目录对文件管理器与数据线可见，用户能取走文件；且属于应用自己的目录，
/// 无需额外权限（区别于需要 MediaStore/SAF 的公共 Downloads）。
/// 若外部存储不可用则回退到内部私有目录（用户不易访问，但至少不丢）。
fn export_dir(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    // 说明：曾尝试写外部专属目录 `Android/data/<pkg>/files/export`（便于用户取走），
    // 但两条路都不通：
    //   1) Rust 直接用 create_dir_all/mkdir —— 被 Android 的 SELinux 拦下，建不出目录
    //   2) 经 JNI 调 `Context.getExternalFilesDir` —— `ndk_context` 在本进程拿不到
    //      Android context，直接 panic（实测把整个 App 带崩，SIGABRT）
    // 因此退回应用内部目录。取文件的办法：adb pull，或后续做「上传到电脑」。
    let internal = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("无法获取应用目录: {}", e))?
        .join("export");
    std::fs::create_dir_all(&internal).map_err(|e| format!("创建导出目录失败: {}", e))?;
    Ok(internal)
}

/// 让网页把诊断信息写进 logcat。
///
/// 手机上没法开控制台，网页端想确认某些能力是否可用（例如 Screen Wake Lock）、
/// 或报告异常时，通过这里落到日志里排查。
#[tauri::command]
fn report_diagnostic(tag: String, detail: String) {
    log::info!("[web:{}] {}", tag, detail);
}

/// 把项目 JSON 写进导出目录，返回落盘后的完整路径。
///
/// 前端已能通过引擎 HTTP 接口拿到导出内容，这里只负责「落到用户能找到的位置」。
/// 文件名做净化处理：去掉路径分隔符，防止 `../` 之类的越界写入。
#[tauri::command]
fn export_project_file(
    app: tauri::AppHandle,
    file_name: String,
    content: String,
) -> Result<String, String> {
    let dir = export_dir(&app)?;

    let safe: String = file_name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '\0' => '_',
            other => other,
        })
        .collect();
    let safe = safe.trim();
    let name = if safe.is_empty() {
        "project-export.json".to_string()
    } else if safe.ends_with(".json") {
        safe.to_string()
    } else {
        format!("{}.json", safe)
    };

    let target = dir.join(&name);
    std::fs::write(&target, content.as_bytes())
        .map_err(|e| format!("写入 {} 失败: {}", target.display(), e))?;
    log::info!("项目已导出到 {}", target.display());
    Ok(target.to_string_lossy().to_string())
}

/// 备份整个数据库文件（`VACUUM INTO`），返回备份文件的完整路径。
///
/// 为什么不用 `std::fs::copy`：数据库开了 WAL，直接拷 `.db` 可能漏掉还在
/// WAL 里的已提交数据（拿到的是不一致的快照）。`VACUUM INTO` 由 SQLite
/// 自己保证一致性，且产出的是一份紧凑的独立库文件。
///
/// 为什么在独立线程里跑自己的 runtime：与 `start_engine` 同样的原因——
/// tauri 的 async runtime 在 Android 目标上会让 sqlx 触发
/// "Send is not general enough" 的生命周期约束冲突。
#[tauri::command]
fn backup_database(app: tauri::AppHandle) -> Result<String, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("无法获取应用数据目录: {}", e))?;
    let db_path = data_dir.join("novel.db");
    if !db_path.exists() {
        return Err(format!("数据库文件不存在: {}", db_path.display()));
    }

    let dir = export_dir(&app)?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let target = dir.join(format!("novel-backup-{}.db", stamp));

    let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();
    let db_str = db_path.to_string_lossy().to_string();
    let target_str = target.to_string_lossy().to_string();

    std::thread::Builder::new()
        .name("novel-backup".into())
        .spawn(move || {
            let result = (|| -> Result<(), String> {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| format!("创建运行时失败: {}", e))?;
                rt.block_on(async {
                    let url = format!("sqlite://{}?mode=rwc", db_str);
                    let pool = sqlx::SqlitePool::connect(&url)
                        .await
                        .map_err(|e| format!("打开数据库失败: {}", e))?;
                    // VACUUM INTO 的目标是字符串字面量，单引号需转义
                    let escaped = target_str.replace('\'', "''");
                    let outcome = sqlx::query(&format!("VACUUM INTO '{}'", escaped))
                        .execute(&pool)
                        .await
                        .map(|_| ())
                        .map_err(|e| format!("VACUUM INTO 失败: {}", e));
                    pool.close().await;
                    outcome
                })
            })();

            let _ = tx.send(result);
        })
        .map_err(|e| format!("启动备份线程失败: {}", e))?;

    let outcome = rx
        .recv()
        .map_err(|e| format!("备份线程无响应: {}", e))?;
    outcome?;

    let size = std::fs::metadata(&target).map(|m| m.len()).unwrap_or(0);
    log::info!("数据库已备份到 {}（{} 字节）", target.display(), size);
    Ok(target.to_string_lossy().to_string())
}

#[derive(serde::Serialize)]
struct ImportOutcome {
    file: String,
    content: String,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_logging();

    tauri::Builder::default()
        // 文件选择（导入电脑端导出的项目 JSON）
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        // 生成期间保持进程存活（Android 前台服务）
        .plugin(keepalive::init())
        .invoke_handler(tauri::generate_handler![
            import_project_file,
            take_schema_rebuild_notice,
            report_diagnostic,
            export_project_file,
            backup_database,
            keepalive::set_keep_alive
        ])
        .setup(|app| {
            log::info!("===== Novel Engine 启动 =====");
            log_paths(app.handle());
            start_engine(app.handle());
            log::info!("===== 入口初始化完成 =====");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
