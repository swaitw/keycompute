//! Claude Provider Adapter
//!
//! Anthropic Claude API 的 Provider 适配器实现。
//! 支持 Claude 3.5 和 Claude 3 系列模型。
//!
//! ## 支持的模型
//! - claude-3-5-sonnet-20241022
//! - claude-3-5-sonnet-20240620
//! - claude-3-5-haiku-20241022
//! - claude-3-opus-20240229
//! - claude-3-sonnet-20240229
//! - claude-3-haiku-20240307
//!
//! ## API 端点
//! 默认: https://api.anthropic.com/v1/messages
//!
//! ## 认证方式
//! 使用 `x-api-key` 头部（而非 Bearer Token）

pub mod adapter;
pub mod protocol;
pub mod stream;

pub use adapter::{CLAUDE_API_VERSION, CLAUDE_DEFAULT_ENDPOINT, CLAUDE_MODELS, ClaudeProvider};
pub use protocol::{
    ClaudeContent, ClaudeError, ClaudeMessage, ClaudeRequest, ClaudeResponse, ClaudeStreamEvent,
    ClaudeStreamMessage, ClaudeUsage, ContentBlock, ContentDelta, MessageDeltaInfo,
};
pub use stream::parse_claude_stream;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_provider_exports() {
        let provider = adapter::ClaudeProvider::new();
        assert_eq!(provider.name(), "claude");
        assert!(!adapter::CLAUDE_MODELS.is_empty());
    }
}
