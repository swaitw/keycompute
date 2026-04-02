//! 时间格式化工具
//!
//! WASM 环境无标准时区支持，不引入外部时间库。
//! 仅做字符串级别的 ISO 8601 解析与格式化。

/// 将 ISO 8601 时间字符串格式化为 `YYYY-MM-DD HH:mm` 供 UI 展示。
///
/// - 输入如 `"2024-03-01T12:34:56Z"` → `"2024-03-01 12:34"`
/// - 输入如 `"2024-03-01T12:34:56.123Z"` → `"2024-03-01 12:34"`
/// - 输入如 `"2024-03-01"` → `"2024-03-01"`
/// - 无法解析时原样返回
pub fn format_time(iso: &str) -> String {
    // ISO 8601 形如 "2024-03-01T12:34:56Z" 或 "2024-03-01T12:34:56.123456Z"
    // 直接按字节切割，不需要任何时区转换
    if let Some(t_pos) = iso.find('T') {
        let date = &iso[..t_pos];
        let rest = &iso[t_pos + 1..];
        // 取时分（前5字节 "HH:mm"）
        let time = if rest.len() >= 5 { &rest[..5] } else { rest };
        format!("{} {}", date, time)
    } else {
        // 纯日期或其他格式，原样返回
        iso.to_string()
    }
}

/// 同 `format_time`，但接受 `Option<&str>`，None 时返回 "—"。
#[allow(dead_code)]
pub fn format_time_opt(iso: Option<&str>) -> String {
    match iso {
        Some(s) if !s.is_empty() => format_time(s),
        _ => "—".to_string(),
    }
}
