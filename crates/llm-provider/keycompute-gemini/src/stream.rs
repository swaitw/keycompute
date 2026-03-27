//! Google Gemini SSE 流解析
//!
//! 将 Google Gemini 的 SSE 流解析为标准化的 StreamEvent
//!
//! Gemini 流式响应格式与 OpenAI 不同：
//! - 使用 JSON 数组形式返回多个事件
//! - 每个事件包含完整的 candidates 结构
//! - 使用 finishReason 表示结束

use futures::{Stream, StreamExt};
use keycompute_provider_trait::ByteStream;
use keycompute_provider_trait::StreamEvent;
use keycompute_provider_trait::stream::sse;
use keycompute_types::{KeyComputeError, Result};
use std::pin::Pin;
use tokio::sync::mpsc;

use crate::protocol::GeminiStreamResponse;

/// 解析 Gemini SSE 流
///
/// 将 HTTP 传输层的字节流转换为标准化的 StreamEvent 流
pub fn parse_gemini_stream(
    stream: ByteStream,
) -> Pin<Box<dyn Stream<Item = Result<StreamEvent>> + Send>> {
    let (tx, rx) = mpsc::channel::<Result<StreamEvent>>(100);

    tokio::spawn(async move {
        let mut buffer = String::new();
        let mut stream = stream;

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
                                let _ = tx.send(Ok(StreamEvent::done())).await;
                                return;
                            }

                            // 解析 JSON 数据
                            match parse_gemini_event(&data) {
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

/// 解析 Gemini 流事件 JSON
fn parse_gemini_event(data: &str) -> Result<Option<StreamEvent>> {
    // Gemini 的流式响应可能是数组形式，需要提取第一个元素
    let response: GeminiStreamResponse = if data.trim_start().starts_with('[') {
        // 尝试解析数组并取第一个元素
        let responses: Vec<GeminiStreamResponse> = serde_json::from_str(data).map_err(|e| {
            KeyComputeError::ProviderError(format!("Failed to parse Gemini stream array: {}", e))
        })?;

        if let Some(first) = responses.first() {
            first.clone()
        } else {
            return Ok(None);
        }
    } else {
        // 单个对象
        serde_json::from_str(data).map_err(|e| {
            KeyComputeError::ProviderError(format!("Failed to parse Gemini stream event: {}", e))
        })?
    };

    // 提取用量信息
    if let Some(usage) = response.usageMetadata {
        return Ok(Some(StreamEvent::usage(
            usage.promptTokenCount,
            usage.candidatesTokenCount,
        )));
    }

    // 处理候选结果和检查是否结束
    if let Some(candidate) = response.candidates.first()
        && let Some(part) = candidate.content.parts.first()
        && let Some(text) = &part.text
        && !text.is_empty()
    {
        return Ok(Some(StreamEvent::delta(text.clone())));
    }

    // 检查是否结束
    if let Some(candidate) = response.candidates.first()
        && let Some(finish_reason) = &candidate.finishReason
    {
        return Ok(Some(StreamEvent::Delta {
            content: String::new(),
            finish_reason: Some(finish_reason.clone()),
        }));
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;

    #[test]
    fn test_parse_gemini_event_with_content() {
        let data = r#"{
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{"text": "Hello"}]
                },
                "finishReason": null
            }]
        }"#;

        let event = parse_gemini_event(data).unwrap();
        assert!(matches!(event, Some(StreamEvent::Delta { content, .. }) if content == "Hello"));
    }

    #[test]
    fn test_parse_gemini_event_with_usage() {
        let data = r#"{
            "candidates": [],
            "usageMetadata": {
                "promptTokenCount": 10,
                "candidatesTokenCount": 20,
                "totalTokenCount": 30
            }
        }"#;

        let event = parse_gemini_event(data).unwrap();
        assert!(matches!(
            event,
            Some(StreamEvent::Usage {
                input_tokens: 10,
                output_tokens: 20
            })
        ));
    }

    #[test]
    fn test_parse_gemini_event_finish() {
        let data = r#"{
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{"text": ""}]
                },
                "finishReason": "STOP"
            }]
        }"#;

        let event = parse_gemini_event(data).unwrap();
        assert!(
            matches!(event, Some(StreamEvent::Delta { content, finish_reason: Some(reason) }) 
            if content.is_empty() && reason == "STOP")
        );
    }

    #[test]
    fn test_parse_gemini_event_array() {
        // Gemini 有时返回数组形式
        let data = r#"[{
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{"text": "Hello from array"}]
                }
            }]
        }]"#;

        let event = parse_gemini_event(data).unwrap();
        assert!(
            matches!(event, Some(StreamEvent::Delta { content, .. }) if content == "Hello from array")
        );
    }

    #[tokio::test]
    async fn test_parse_gemini_stream() {
        // 模拟 SSE 数据
        let sse_data = vec![
            Ok(bytes::Bytes::from(
                "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"Hello\"}]}}]}\n\n",
            )),
            Ok(bytes::Bytes::from(
                "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\" World\"}]}}]}\n\n",
            )),
            Ok(bytes::Bytes::from(
                "data: {\"candidates\":[],\"usageMetadata\":{\"promptTokenCount\":10,\"candidatesTokenCount\":2,\"totalTokenCount\":12}}\n\n",
            )),
        ];

        let byte_stream: ByteStream = Box::pin(stream::iter(sse_data));
        let mut stream = parse_gemini_stream(byte_stream);

        // 收集所有事件
        let mut events = Vec::new();
        while let Some(result) = stream.next().await {
            if let Ok(event) = result {
                events.push(event);
            }
        }

        // 验证事件序列
        assert!(events.len() >= 2); // 至少有 delta + usage + done
        assert!(matches!(&events[0], StreamEvent::Delta { content, .. } if content == "Hello"));
        assert!(matches!(&events[1], StreamEvent::Delta { content, .. } if content == " World"));
    }
}
