//! Schema 漂移自动校验（离线静态测试，不需要数据库）。
//!
//! 背景：本项目多次发生 migrations ↔ Rust SQL 的漂移
//! （character_state 缺列、faction_profile 列数不匹配、real_name→name 重命名漏改等）。
//!
//! 本测试从 `crates/db/migrations/*.sql` 推导出每张表的真实列集合（按文件名顺序回放
//! CREATE TABLE / ADD COLUMN / RENAME COLUMN / DROP COLUMN / RENAME TO），
//! 然后扫描 `crates/db/src` 下所有 Rust 源码字符串字面量中的 SQL：
//!   1. FROM / JOIN / INSERT INTO / UPDATE 引用的表必须存在；
//!   2. INSERT INTO t (a, b, c) 的每一列必须存在于 t 的推导列集合中。
//!
//! 若失败，说明代码引用了迁移里不存在的表/列 —— 正是历史上那类 500 错误的根源。

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const MIGRATIONS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/migrations");
const SRC_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src");

/// ---------- migrations 解析 ----------

fn migration_files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(MIGRATIONS_DIR)
        .expect("migrations dir must exist")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "sql").unwrap_or(false))
        .collect();
    files.sort();
    assert!(
        !files.is_empty(),
        "no migration files found under {}",
        MIGRATIONS_DIR
    );
    files
}

fn is_constraint_keyword(tok: &str) -> bool {
    matches!(
        tok.to_ascii_uppercase().as_str(),
        "PRIMARY" | "FOREIGN" | "UNIQUE" | "CHECK" | "CONSTRAINT" | "KEY" | "INDEX"
    )
}

/// 标识符清洗：去引号、去结尾分号，并截掉 `name(id)` / `name(` 这类后缀
fn strip_ident(tok: &str) -> String {
    let t = tok.trim_matches('"').trim_matches('`');
    let t = t.trim_end_matches(';');
    match t.find('(') {
        Some(p) => t[..p].to_string(),
        None => t.to_string(),
    }
}

/// 按 top-level 逗号拆分列定义体（忽略括号内的逗号，如 VARCHAR(50)、NUMERIC(10,2)）
fn split_top_level(body: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();
    for ch in body.chars() {
        match ch {
            '(' => {
                depth += 1;
                cur.push(ch);
            }
            ')' => {
                depth -= 1;
                cur.push(ch);
            }
            ',' if depth == 0 => {
                parts.push(cur.clone());
                cur.clear();
            }
            _ => cur.push(ch),
        }
    }
    if !cur.trim().is_empty() {
        parts.push(cur);
    }
    parts
}

/// 回放全部迁移，得到 表 -> 列集合
fn derive_schema() -> BTreeMap<String, BTreeSet<String>> {
    let mut schema: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for file in migration_files() {
        let sql = fs::read_to_string(&file).expect("read migration");
        let mut lines = sql.lines().peekable();

        while let Some(line) = lines.next() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("--") {
                continue; // 注释行
            }
            let upper = line.to_ascii_uppercase();

            // CREATE TABLE [IF NOT EXISTS] name ( ... );
            // 必须是语句起始（注释里出现 "CREATE TABLE" 字样不算）
            if upper.trim_start().starts_with("CREATE TABLE") {
                let mut rest = line.split_ascii_whitespace().peekable();
                while let Some(w) = rest.next() {
                    if w.eq_ignore_ascii_case("TABLE") {
                        // 跳过可选 IF NOT EXISTS
                        let mut name_tok = rest.next().unwrap_or("").to_string();
                        if name_tok.eq_ignore_ascii_case("IF") {
                            rest.next(); // NOT
                            rest.next(); // EXISTS
                            name_tok = rest.next().unwrap_or("").to_string();
                        }
                        let table = strip_ident(name_tok.trim_end_matches('('));
                        // 收集表体直到 ");"
                        let mut body = String::new();
                        for l in lines.by_ref() {
                            let t = l.trim();
                            if t.starts_with("--") {
                                continue;
                            }
                            if t.starts_with(");") || t == ")" {
                                break;
                            }
                            body.push_str(l);
                            body.push('\n');
                        }
                        let cols = schema.entry(table).or_default();
                        for def in split_top_level(&body) {
                            let d = def.trim();
                            if d.is_empty() {
                                continue;
                            }
                            let first = d.split_ascii_whitespace().next().unwrap_or("");
                            let first_clean = strip_ident(first.trim_end_matches('('));
                            if first_clean.is_empty()
                                || is_constraint_keyword(&first_clean)
                            {
                                continue;
                            }
                            cols.insert(first_clean);
                        }
                        break;
                    }
                }
                continue;
            }

            // ALTER TABLE ...
            if upper.contains("ALTER TABLE") {
                let tokens: Vec<&str> = line.split_ascii_whitespace().collect();
                if let Some(pos) = tokens
                    .iter()
                    .position(|w| w.eq_ignore_ascii_case("TABLE"))
                {
                    if let Some(table_raw) = tokens.get(pos + 1) {
                        let table = strip_ident(table_raw);
                        let rest: Vec<&str> = tokens[pos + 2..].to_vec();
                        let rest_joined = rest.join(" ");
                        let rest_upper = rest_joined.to_ascii_uppercase();

                        if rest_upper.starts_with("RENAME TO") {
                            if let Some(new) = rest.get(2) {
                                if let Some(entry) = schema.remove(&table) {
                                    schema.insert(strip_ident(new), entry);
                                }
                            }
                        } else if rest_upper.starts_with("RENAME COLUMN") {
                            // RENAME COLUMN old TO new
                            if let (Some(old), Some(new)) = (rest.get(2), rest.get(4)) {
                                if rest
                                    .get(3)
                                    .map(|w| w.eq_ignore_ascii_case("TO"))
                                    .unwrap_or(false)
                                {
                                    if let Some(cols) = schema.get_mut(&table) {
                                        if cols.remove(&strip_ident(old)) {
                                            cols.insert(strip_ident(new));
                                        }
                                    }
                                }
                            }
                        } else if rest_upper.starts_with("DROP COLUMN") {
                            if let Some(col) = rest.get(2) {
                                let col = strip_ident(col.trim_end_matches(';'));
                                if col.eq_ignore_ascii_case("IF") {
                                    // DROP COLUMN IF EXISTS col
                                    if let Some(c) = rest.get(4) {
                                        if let Some(cols) = schema.get_mut(&table) {
                                            cols.remove(&strip_ident(c));
                                        }
                                    }
                                } else if let Some(cols) = schema.get_mut(&table) {
                                    cols.remove(&col);
                                }
                            }
                        } else if rest_upper.starts_with("ADD COLUMN")
                            || (rest_upper.starts_with("ADD ")
                                && !rest_upper.starts_with("ADD CONSTRAINT")
                                && !rest_upper.starts_with("ADD FOREIGN")
                                && !rest_upper.starts_with("ADD PRIMARY")
                                && !rest_upper.starts_with("ADD UNIQUE")
                                && !rest_upper.starts_with("ADD CHECK"))
                        {
                            // ADD [COLUMN] [IF NOT EXISTS] col type...
                            let mut idx = if rest_upper.starts_with("ADD COLUMN") {
                                2
                            } else {
                                1
                            };
                            if rest
                                .get(idx)
                                .map(|w| w.eq_ignore_ascii_case("IF"))
                                .unwrap_or(false)
                            {
                                idx += 3; // IF NOT EXISTS
                            }
                            if let Some(col) = rest.get(idx) {
                                let col = strip_ident(col);
                                if !col.is_empty() && !is_constraint_keyword(&col) {
                                    schema.entry(table).or_default().insert(col);
                                }
                            }
                        }
                    }
                }
                continue;
            }
        }
    }

    schema
}

/// ---------- Rust 源码 SQL 扫描 ----------

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in fs::read_dir(dir).expect("src dir exists").flatten() {
        let p = e.path();
        if p.is_dir() {
            // bin/ 是一次性迁移工具，不在校验范围
            if p.file_name().map(|n| n == "bin").unwrap_or(false) {
                continue;
            }
            rust_files(&p, out);
        } else if p.extension().map(|x| x == "rs").unwrap_or(false) {
            out.push(p);
        }
    }
}

/// 提取双引号字符串字面量内容（处理 \" 转义；本 crate 无需处理原始字符串于 src/）
fn string_literals(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '"' {
            let mut s = String::new();
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    i += 1;
                    s.push(chars[i]);
                } else {
                    s.push(chars[i]);
                }
                i += 1;
            }
            out.push(s);
        }
        i += 1;
    }
    out
}

/// 收集一条 SQL 文本里的 CTE 名（WITH [RECURSIVE] name AS (...)），避免误报
fn cte_names(sql: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let tokens: Vec<&str> = sql.split_ascii_whitespace().collect();
    for (i, w) in tokens.iter().enumerate() {
        if w.eq_ignore_ascii_case("WITH") || w.eq_ignore_ascii_case("AS") {
            // WITH RECURSIVE sub AS / ... ) AS sub (
            if let Some(next) = tokens.get(i + 1) {
                if w.eq_ignore_ascii_case("WITH")
                    && next.eq_ignore_ascii_case("RECURSIVE")
                {
                    if let Some(n) = tokens.get(i + 2) {
                        names.insert(strip_ident(n.trim_end_matches('(')));
                    }
                } else if w.eq_ignore_ascii_case("WITH") {
                    names.insert(strip_ident(next.trim_end_matches('(')));
                } else if w.eq_ignore_ascii_case("AS") {
                    // `) AS name (` 形式：name 后跟 (
                    if let Some(n) = tokens.get(i + 1) {
                        if n.starts_with('(') {
                            // AS ( 子查询，跳过
                        } else {
                            names.insert(strip_ident(n));
                        }
                    }
                }
            }
        }
    }
    names
}

/// 表名位置的保留字（如 `DO UPDATE SET ...` 中的 SET），不是表
fn is_sql_keyword_ident(tok: &str) -> bool {
    matches!(
        tok.to_ascii_uppercase().as_str(),
        "SET" | "VALUES" | "SELECT" | "WHERE" | "RETURNING" | "ONLY" | "DEFAULT"
            | "AS" | "ON" | "AND" | "OR" | "NOT" | "NULL" | "LATERAL" | "UNNEST"
    )
}

#[test]
fn rust_sql_references_match_migrations() {
    let schema = derive_schema();
    assert!(schema.contains_key("project"), "sanity: project table derived");
    assert!(
        schema.contains_key("novel_state_snapshot"),
        "sanity: novel_state_snapshot derived"
    );

    let mut files = Vec::new();
    rust_files(Path::new(SRC_DIR), &mut files);
    assert!(files.len() > 30, "expected to scan many rs files");

    let mut problems: Vec<String> = Vec::new();

    for f in &files {
        let src = fs::read_to_string(f).expect("read rs file");
        for lit in string_literals(&src) {
            // 只对看起来含 SQL 关键字的字面量做检查
            let upper = lit.to_ascii_uppercase();
            if !(upper.contains(" SELECT ") || upper.contains("INSERT INTO") || upper.contains(" UPDATE ") || upper.contains(" JOIN ") || upper.starts_with("SELECT") || upper.starts_with("INSERT") || upper.starts_with("UPDATE") || upper.starts_with("DELETE")) {
                continue;
            }

            let ctes = cte_names(&lit);

            // 表引用：FROM / JOIN / INSERT INTO / UPDATE <ident>
            // 大小写敏感：本项目 SQL 关键字一律大写；小写 "update ..." 是错误信息等散文
            for kw in ["FROM", "JOIN", "INTO", "UPDATE"] {
                let mut search = 0usize;
                while let Some(rel) = lit[search..].find(kw) {
                    let abs_kw = search + rel;
                    // 关键词必须是完整词
                    let before_ok = abs_kw == 0
                        || !lit
                            .as_bytes()
                            .get(abs_kw - 1)
                            .map(|c| c.is_ascii_alphabetic())
                            .unwrap_or(false);
                    let after_pos = abs_kw + kw.len();
                    let after_ok = after_pos >= lit.len()
                        || !lit
                            .as_bytes()
                            .get(after_pos)
                            .map(|c| c.is_ascii_alphabetic())
                            .unwrap_or(false);
                    search = after_pos.max(search + 1);

                    // 对 UPDATE 需要排除 "FOR UPDATE"（行锁语法，后面不跟表名）
                    if kw == "UPDATE" && abs_kw >= 4 {
                        let prev = &upper[abs_kw.saturating_sub(4)..abs_kw];
                        if prev == "FOR " {
                            continue;
                        }
                    }
                    if !before_ok || !after_ok {
                        continue;
                    }

                    let tail = lit[after_pos..].trim_start();
                    let ident: String = tail
                        .chars()
                        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                        .collect();
                    if ident.is_empty() {
                        continue;
                    }
                    // 跳过 schema 限定名、函数调用与保留字
                    if tail[ident.len()..].starts_with('.') || tail[ident.len()..].starts_with('(') {
                        continue;
                    }
                    if is_sql_keyword_ident(&ident) {
                        continue;
                    }
                    if ctes.contains(&ident) {
                        continue;
                    }
                    if !schema.contains_key(&ident) {
                        problems.push(format!(
                            "{}: {:?} 引用了迁移中不存在的表 `{}`",
                            f.display(),
                            truncate(&lit),
                            ident
                        ));
                    }
                }
            }

            // INSERT 列清单
            let bytes = lit.as_bytes();
            let mut idx = 0usize;
            while let Some(p) = upper[idx..].find("INSERT INTO") {
                let start = idx + p;
                let after = &lit[start + "INSERT INTO".len()..];
                let t_trim = after.trim_start();
                let table: String = t_trim
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();
                if !table.is_empty() {
                    if let Some(rest) = t_trim.get(table.len()..) {
                        if let Some(open) = rest.find('(') {
                            if let Some(close) = rest[open..].find(')') {
                                let cols_raw = &rest[open + 1..open + close];
                                if let Some(cols) = schema.get(&table) {
                                    for c in cols_raw.split(',') {
                                        let c = strip_ident(c.trim());
                                        if c.is_empty() {
                                            continue;
                                        }
                                        if !cols.contains(&c) {
                                            problems.push(format!(
                                                "{}: INSERT INTO {} 引用了不存在的列 `{}`（现有列: {:?}）",
                                                f.display(),
                                                table,
                                                c,
                                                cols
                                            ));
                                        }
                                    }
                                } else {
                                    problems.push(format!(
                                        "{}: INSERT INTO 不存在的表 `{}`",
                                        f.display(),
                                        table
                                    ));
                                }
                            }
                        }
                    }
                }
                idx = start + "INSERT INTO".len();
            }
            let _ = bytes;
        }
    }

    if !problems.is_empty() {
        panic!(
            "发现 {} 处 SQL 与 migrations 漂移:\n{}",
            problems.len(),
            problems.join("\n")
        );
    }
}

fn truncate(s: &str) -> String {
    if s.len() > 80 {
        format!("{}...", &s[..80])
    } else {
        s.to_string()
    }
}
