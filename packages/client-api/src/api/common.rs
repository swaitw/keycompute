//! 公共响应类型
//!
//! 供多个 API 模块复用的通用数据结构

use serde::Deserialize;

/// 通用消息响应
///
/// 后端返回 `{ "message": "..." }` 格式的接口统一使用此类型。
#[derive(Debug, Clone, Deserialize)]
pub struct MessageResponse {
    pub message: String,
}
