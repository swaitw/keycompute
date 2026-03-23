//! 模拟 Provider Adapter

use async_trait::async_trait;
use futures::stream;
use keycompute_provider_trait::{
    HttpTransport, ProviderAdapter, StreamBox, StreamEvent, UpstreamRequest,
};
use keycompute_types::KeyComputeError;
use std::sync::Mutex;

/// 模拟 Provider
#[derive(Debug)]
pub struct MockProvider {
    name: &'static str,
    supported_models: Vec<&'static str>,
    response_chunks: Mutex<Vec<String>>,
    input_tokens: u32,
    output_tokens: u32,
    should_fail: bool,
}

impl MockProvider {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            supported_models: vec!["gpt-4o", "gpt-3.5-turbo"],
            response_chunks: Mutex::new(vec![
                "Hello".to_string(),
                " from".to_string(),
                " mock".to_string(),
            ]),
            input_tokens: 10,
            output_tokens: 3,
            should_fail: false,
        }
    }

    pub fn with_chunks(mut self, chunks: Vec<String>) -> Self {
        let len = chunks.len();
        *self.response_chunks.lock().unwrap() = chunks;
        self.output_tokens = len as u32;
        self
    }

    pub fn with_tokens(mut self, input: u32, output: u32) -> Self {
        self.input_tokens = input;
        self.output_tokens = output;
        self
    }

    pub fn with_failure(mut self) -> Self {
        self.should_fail = true;
        self
    }

    pub fn with_models(mut self, models: Vec<&'static str>) -> Self {
        self.supported_models = models;
        self
    }
}

#[async_trait]
impl ProviderAdapter for MockProvider {
    fn name(&self) -> &'static str {
        self.name
    }

    fn supported_models(&self) -> Vec<&'static str> {
        self.supported_models.clone()
    }

    async fn stream_chat(
        &self,
        _transport: &dyn HttpTransport,
        _request: UpstreamRequest,
    ) -> keycompute_types::Result<StreamBox> {
        if self.should_fail {
            return Err(KeyComputeError::ProviderError("Mock failure".to_string()));
        }

        let chunks = self.response_chunks.lock().unwrap().clone();
        let input_tokens = self.input_tokens;
        let output_tokens = self.output_tokens;

        let stream = stream::unfold(
            (chunks, 0usize, input_tokens, output_tokens),
            |(chunks, index, input, output)| async move {
                if index >= chunks.len() {
                    // 发送 Usage 事件后结束
                    if index == chunks.len() {
                        let event = StreamEvent::Usage {
                            input_tokens: input,
                            output_tokens: output,
                        };
                        return Some((Ok(event), (chunks, index + 1, input, output)));
                    }
                    // 发送 Done 事件
                    if index == chunks.len() + 1 {
                        let event = StreamEvent::Done;
                        return Some((Ok(event), (chunks, index + 1, input, output)));
                    }
                    return None;
                }

                let content = chunks[index].clone();
                let event = StreamEvent::Delta {
                    content,
                    finish_reason: None,
                };

                Some((Ok(event), (chunks, index + 1, input, output)))
            },
        );

        Ok(Box::pin(stream))
    }
}

/// 创建模拟 Provider 的工厂
pub struct MockProviderFactory;

impl MockProviderFactory {
    /// 创建一个成功的 OpenAI 模拟 Provider
    pub fn create_openai() -> MockProvider {
        MockProvider::new("openai")
            .with_models(vec!["gpt-4o", "gpt-4o-mini", "gpt-3.5-turbo"])
            .with_chunks(vec![
                "Hello".to_string(),
                " from".to_string(),
                " OpenAI".to_string(),
            ])
            .with_tokens(10, 3)
    }

    /// 创建一个成功的 Anthropic 模拟 Provider
    pub fn create_anthropic() -> MockProvider {
        MockProvider::new("anthropic")
            .with_models(vec!["claude-3-opus", "claude-3-sonnet"])
            .with_chunks(vec![
                "Hello".to_string(),
                " from".to_string(),
                " Claude".to_string(),
            ])
            .with_tokens(8, 3)
    }

    /// 创建一个会失败的 Provider（用于测试 fallback）
    pub fn create_failing() -> MockProvider {
        MockProvider::new("failing").with_failure()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_mock_provider_stream() {
        let provider = MockProviderFactory::create_openai();
        let request = UpstreamRequest::new("http://test", "test-key", "gpt-4o");

        // 使用默认 HTTP 传输层
        let transport = keycompute_provider_trait::DefaultHttpTransport::new();
        let mut stream: keycompute_provider_trait::StreamBox =
            provider.stream_chat(&transport, request).await.unwrap();
        let mut events = Vec::new();

        while let Some(event) = stream.next().await {
            events.push(event.unwrap());
        }

        // 应该有 3 个 Delta + 1 个 Usage + 1 个 Done = 5 个事件
        assert_eq!(events.len(), 5);
    }

    #[tokio::test]
    async fn test_mock_provider_failure() {
        let provider = MockProviderFactory::create_failing();
        let request = UpstreamRequest::new("http://test", "test-key", "gpt-4o");

        // 使用默认 HTTP 传输层
        let transport = keycompute_provider_trait::DefaultHttpTransport::new();
        let result: Result<keycompute_provider_trait::StreamBox, _> =
            provider.stream_chat(&transport, request).await;
        assert!(result.is_err());
    }
}
