//! Claude SSE 流解析
//!
//! 将 Anthropic Claude 的 SSE 流解析为标准化的 StreamEvent

use futures::{Stream, StreamExt};
use keycompute_provider_trait::ByteStream;
use keycompute_provider_trait::StreamEvent;
use keycompute_provider_trait::stream::sse;
use keycompute_types::{KeyComputeError, Result};
use std::pin::Pin;
use tokio::sync::mpsc;

use crate::protocol::ClaudeStreamEvent;

/// 解析 Claude SSE 流
///
/// 将 HTTP 传输层的字节流转换为标准化的 StreamEvent 流
pub fn parse_claude_stream(
    stream: ByteStream,
) -> Pin<Box<dyn Stream<Item = Result<StreamEvent>> + Send>> {
    let (tx, rx) = mpsc::channel::<Result<StreamEvent>>(100);

    tokio::spawn(async move {
        let mut buffer = String::new();
        let mut stream = stream;
        let mut accumulated_text = String::new();

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    let text = String::from_utf8_lossy(&chunk);
                    buffer.push_str(&text);

                    // 处理缓冲区中的完整行
                    while let Some(pos) = buffer.find('\n') {
                        let line = buffer[..pos].to_string();
                        buffer.drain(..=pos);

                        // 处理可能的 \r\n
                        let line = line.trim_end_matches('\r');

                        if let Some(data) = sse::parse_sse_line(line) {
                            if sse::is_done_marker(&data) {
                                // Claude 不使用 [DONE] 标记，而是使用 message_stop 事件
                                continue;
                            }

                            // 解析 JSON 数据
                            match parse_claude_event(&data, &mut accumulated_text) {
                                Ok(Some(event)) => {
                                    if tx.send(Ok(event)).await.is_err() {
                                        return;
                                    }
                                }
                                Ok(None) => continue,
                                Err(e) => {
                                    let _ = tx.send(Err(e)).await;
                                    return;
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    let _ = tx
                        .send(Err(KeyComputeError::ProviderError(e.to_string())))
                        .await;
                    return;
                }
            }
        }

        // 流结束
        let _ = tx.send(Ok(StreamEvent::done())).await;
    });

    Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx))
}

/// 解析 Claude 流事件 JSON
fn parse_claude_event(data: &str, accumulated_text: &mut String) -> Result<Option<StreamEvent>> {
    let event: ClaudeStreamEvent = serde_json::from_str(data).map_err(|e| {
        KeyComputeError::ProviderError(format!("Failed to parse Claude stream event: {}", e))
    })?;

    match event {
        ClaudeStreamEvent::MessageStart { message } => {
            // 消息开始，可以记录用量信息（输入 tokens）
            if message.usage.input_tokens > 0 {
                return Ok(Some(StreamEvent::usage(
                    message.usage.input_tokens,
                    0, // 输出 tokens 还未知
                )));
            }
            Ok(None)
        }
        ClaudeStreamEvent::ContentBlockStart { content_block, .. } => {
            // 内容块开始
            if let crate::protocol::ContentBlock::Text { text } = content_block
                && !text.is_empty()
            {
                accumulated_text.push_str(&text);
                return Ok(Some(StreamEvent::delta(text)));
            }
            Ok(None)
        }
        ClaudeStreamEvent::ContentBlockDelta { delta, .. } => {
            // 内容增量
            match delta {
                crate::protocol::ContentDelta::TextDelta { text } => {
                    accumulated_text.push_str(&text);
                    Ok(Some(StreamEvent::delta(text)))
                }
            }
        }
        ClaudeStreamEvent::ContentBlockStop { .. } => {
            // 内容块结束，无需特殊处理
            Ok(None)
        }
        ClaudeStreamEvent::MessageDelta { delta, usage } => {
            // 消息增量，包含停止原因和用量信息
            if let Some(usage) = usage {
                // 返回最终的用量信息
                return Ok(Some(StreamEvent::usage(
                    0, // 输入 tokens 已经在 message_start 中报告
                    usage.output_tokens,
                )));
            }

            // 如果有停止原因，发送一个带 finish_reason 的 delta
            if delta.stop_reason.is_some() {
                return Ok(Some(StreamEvent::Delta {
                    content: String::new(),
                    finish_reason: delta.stop_reason,
                }));
            }

            Ok(None)
        }
        ClaudeStreamEvent::MessageStop => {
            // 消息结束
            Ok(Some(StreamEvent::done()))
        }
        ClaudeStreamEvent::Error { error } => {
            // 错误事件
            Ok(Some(StreamEvent::error(format!(
                "Claude API error ({}): {}",
                error.r#type, error.message
            ))))
        }
        ClaudeStreamEvent::Ping => {
            // Ping 事件，忽略
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;

    #[test]
    fn test_parse_claude_event_message_start() {
        let data = r#"{"type": "message_start", "message": {"id": "msg_01XgYhR8f4h3n7sY3R4j4V3d", "type": "message", "role": "assistant", "model": "claude-3-5-sonnet-20241022", "usage": {"input_tokens": 10, "output_tokens": 0}}}"#;
        let mut accumulated = String::new();
        let event = parse_claude_event(data, &mut accumulated).unwrap();

        assert!(matches!(
            event,
            Some(StreamEvent::Usage {
                input_tokens: 10,
                output_tokens: 0
            })
        ));
    }

    #[test]
    fn test_parse_claude_event_text_delta() {
        let data = r#"{"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "Hello"}}"#;
        let mut accumulated = String::new();
        let event = parse_claude_event(data, &mut accumulated).unwrap();

        assert!(matches!(event, Some(StreamEvent::Delta { content, .. }) if content == "Hello"));
        assert_eq!(accumulated, "Hello");
    }

    #[test]
    fn test_parse_claude_event_message_stop() {
        let data = r#"{"type": "message_stop"}"#;
        let mut accumulated = String::new();
        let event = parse_claude_event(data, &mut accumulated).unwrap();

        assert!(matches!(event, Some(StreamEvent::Done)));
    }

    #[test]
    fn test_parse_claude_event_error() {
        let data = r#"{"type": "error", "error": {"type": "rate_limit_error", "message": "Rate limit exceeded"}}"#;
        let mut accumulated = String::new();
        let event = parse_claude_event(data, &mut accumulated).unwrap();

        assert!(matches!(event, Some(StreamEvent::Error { message }) 
            if message.contains("rate_limit_error")));
    }

    #[test]
    fn test_parse_claude_event_message_delta_with_usage() {
        let data = r#"{"type": "message_delta", "delta": {"stop_reason": "end_turn"}, "usage": {"output_tokens": 50}}"#;
        let mut accumulated = String::new();
        let event = parse_claude_event(data, &mut accumulated).unwrap();

        assert!(matches!(
            event,
            Some(StreamEvent::Usage {
                output_tokens: 50,
                ..
            })
        ));
    }

    #[tokio::test]
    async fn test_parse_claude_stream() {
        // 模拟 SSE 数据
        let sse_data = vec![
            Ok(bytes::Bytes::from(
                "data: {\"type\": \"message_start\", \"message\": {\"id\": \"msg_01\", \"type\": \"message\", \"role\": \"assistant\", \"model\": \"claude-3-5-sonnet\", \"usage\": {\"input_tokens\": 10, \"output_tokens\": 0}}}\n\n",
            )),
            Ok(bytes::Bytes::from(
                "data: {\"type\": \"content_block_delta\", \"index\": 0, \"delta\": {\"type\": \"text_delta\", \"text\": \"Hello\"}}\n\n",
            )),
            Ok(bytes::Bytes::from(
                "data: {\"type\": \"content_block_delta\", \"index\": 0, \"delta\": {\"type\": \"text_delta\", \"text\": \" World\"}}\n\n",
            )),
            Ok(bytes::Bytes::from(
                "data: {\"type\": \"message_delta\", \"delta\": {\"stop_reason\": \"end_turn\"}, \"usage\": {\"output_tokens\": 2}}\n\n",
            )),
            Ok(bytes::Bytes::from("data: {\"type\": \"message_stop\"}\n\n")),
        ];

        let byte_stream: ByteStream = Box::pin(stream::iter(sse_data));
        let mut stream = parse_claude_stream(byte_stream);

        // 收集所有事件
        let mut events = Vec::new();
        while let Some(result) = stream.next().await {
            if let Ok(event) = result {
                events.push(event);
            }
        }

        // 验证事件序列
        assert!(events.len() >= 3); // usage + delta + delta + usage + done
        assert!(matches!(
            &events[0],
            StreamEvent::Usage {
                input_tokens: 10,
                ..
            }
        ));
        assert!(matches!(&events[1], StreamEvent::Delta { content, .. } if content == "Hello"));
        assert!(matches!(&events[2], StreamEvent::Delta { content, .. } if content == " World"));
    }
}
