//! 构建脚本：把电脑端的 AI 配置注入编译环境变量。
//!
//! 目的：手机端首次启动即为「已配置」状态，用户不用在手机上重新敲一遍
//! 供应商信息（接口地址 / 密钥 / 模型 / 输出上限）。这些值只在数据库里
//! **没有**配置时写入一次，之后以用户在设置页改的值为准，不会被覆盖。
//!
//! 取值顺序：进程环境变量 → 仓库根目录 `.env` → 内置默认值。
//! 未提供密钥时留空，此时 App 启动后走首启引导页让用户手填。

/// 从仓库根目录 `.env` 读取一个 key（文件或 key 不存在时返回 None）。
fn from_dotenv(key: &str) -> Option<String> {
    // 仓库根目录是 mobile/src-tauri/../../
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()?
        .parent()?
        .join(".env");
    let content = std::fs::read_to_string(path).ok()?;
    let prefix = format!("{}=", key);
    for line in content.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix(&prefix) {
            let v = v.trim().trim_matches('"');
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

/// 先看进程环境变量，再回落到 `.env`，最后用默认值。
fn resolve(key: &str, fallback: &str) -> String {
    std::env::var(key)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| from_dotenv(key))
        .unwrap_or_else(|| fallback.to_string())
}

fn main() {
    let mut key = resolve("OPENCODE_API_KEY", "");
    let base = resolve("OPENCODE_BASE_URL", "https://opencode.ai/zen/go/v1");
    let model = resolve("OPENCODE_MODEL", "deepseek-flash");
    let max_out = resolve("OPENCODE_MAX_OUTPUT_TOKENS", "22000");

    // 测试开关：设置 NOVEL_DISABLE_BOOT_KEY=1 可以构建出「不带默认 Key」的包，
    // 用于验证首次配置引导页（正常构建不要设置它）。
    if std::env::var("NOVEL_DISABLE_BOOT_KEY").as_deref() == Ok("1") {
        key.clear();
        println!("cargo:warning=已按 NOVEL_DISABLE_BOOT_KEY 清空默认 API Key（用于验证首启引导）");
    }

    println!("cargo:rustc-env=NOVEL_BOOT_API_KEY={}", key);
    println!("cargo:rustc-env=NOVEL_BOOT_BASE_URL={}", base);
    println!("cargo:rustc-env=NOVEL_BOOT_MODEL={}", model);
    println!("cargo:rustc-env=NOVEL_BOOT_MAX_OUTPUT_TOKENS={}", max_out);
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../../.env");
    println!("cargo:rerun-if-env-changed=NOVEL_DISABLE_BOOT_KEY");

    tauri_build::build()
}
