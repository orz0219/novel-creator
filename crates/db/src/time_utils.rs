//! Time utilities for DuckDB

use chrono::{DateTime, NaiveDateTime, Utc};

/// 从 DuckDB 的字符串字段解析 DateTime<Utc>，失败返回当前时间
pub fn parse_timestamp(s: &str) -> DateTime<Utc> {
    if s.is_empty() {
        return Utc::now();
    }
    // DuckDB returns timestamps as "YYYY-MM-DD HH:MM:SS.ffffff"
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f")
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S"))
        .map(|ndt| ndt.and_utc())
        .unwrap_or(Utc::now())
}

/// 从 DuckDB Row 读取时间戳列（用于 query_map 闭包内，不返回 Result）
pub fn get_timestamp(row: &duckdb::Row, idx: usize) -> DateTime<Utc> {
    let s: String = row.get(idx).unwrap_or_default();
    parse_timestamp(&s)
}

/// 从 DuckDB Row 读取可选时间戳列（用于 query_map 闭包内）
pub fn get_optional_timestamp(row: &duckdb::Row, idx: usize) -> Option<DateTime<Utc>> {
    let s: Option<String> = row.get(idx).unwrap_or(None);
    s.filter(|s| !s.is_empty()).map(|s| parse_timestamp(&s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_timestamp() {
        // Test various timestamp formats
        let ts1 = parse_timestamp("2024-01-15 10:30:00");
        assert_eq!(ts1.format("%Y-%m-%d %H:%M:%S").to_string(), "2024-01-15 10:30:00");
        
        // Test DuckDB's default timestamp format (with microseconds)
        let ts2 = parse_timestamp("2024-01-15 10:30:00.123456");
        assert_eq!(ts2.format("%Y-%m-%d %H:%M:%S").to_string(), "2024-01-15 10:30:00");
        
        // Test empty string returns current time
        let ts3 = parse_timestamp("");
        assert!(ts3.signed_duration_since(Utc::now()).num_seconds().abs() < 5);
    }

    #[test]
    fn test_parse_empty() {
        let ts = parse_timestamp("");
        // Should return current time (approximately)
        assert!(ts.signed_duration_since(Utc::now()).num_seconds().abs() < 5);
    }
}
