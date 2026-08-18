//! Time utilities for PostgreSQL
//!
//! PostgreSQL 返回原生时间类型，sqlx 直接映射为 chrono::DateTime<Utc>。
//! 保留字符串解析函数用于兼容旧数据迁移场景。

use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::Row;

/// 从字符串解析 DateTime<Utc>，失败返回当前时间
///
/// 主要用于数据迁移时处理旧的字符串格式时间戳。
pub fn parse_timestamp(s: &str) -> DateTime<Utc> {
    if s.is_empty() {
        return Utc::now();
    }
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f")
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S"))
        .map(|ndt| ndt.and_utc())
        .unwrap_or(Utc::now())
}

/// 从 sqlx 行读取时间戳列
///
/// PostgreSQL 原生返回 TIMESTAMPTZ，sqlx 直接映射为 DateTime<Utc>。
/// 此函数处理 Option<DateTime<Utc>> 的常见情况。
pub fn get_timestamp_from_row(row: &sqlx::postgres::PgRow, column: &str) -> DateTime<Utc> {
    row.try_get::<DateTime<Utc>, _>(column)
        .unwrap_or_else(|_| Utc::now())
}

/// 从 sqlx 行读取可选时间戳列
pub fn get_optional_timestamp_from_row(
    row: &sqlx::postgres::PgRow,
    column: &str,
) -> Option<DateTime<Utc>> {
    row.try_get::<DateTime<Utc>, _>(column).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_timestamp() {
        let ts1 = parse_timestamp("2024-01-15 10:30:00");
        assert_eq!(
            ts1.format("%Y-%m-%d %H:%M:%S").to_string(),
            "2024-01-15 10:30:00"
        );

        let ts2 = parse_timestamp("2024-01-15 10:30:00.123456");
        assert_eq!(
            ts2.format("%Y-%m-%d %H:%M:%S").to_string(),
            "2024-01-15 10:30:00"
        );

        let ts3 = parse_timestamp("");
        assert!(ts3.signed_duration_since(Utc::now()).num_seconds().abs() < 5);
    }

    #[test]
    fn test_parse_empty() {
        let ts = parse_timestamp("");
        assert!(ts.signed_duration_since(Utc::now()).num_seconds().abs() < 5);
    }
}
