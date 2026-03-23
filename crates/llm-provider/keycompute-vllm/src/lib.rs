//! vLLM Provider Adapter
//!
//! vLLM 是一个高性能的 LLM 推理引擎，提供 OpenAI 兼容的 API。
//! 本模块复用 OpenAI 协议层实现。
//!
//! # 支持的模型
//! vLLM 支持加载任意 HuggingFace 模型，常见示例：
//! - `meta-llama/Llama-3.1-8B-Instruct`
//! - `Qwen/Qwen2.5-7B-Instruct`
//! - `mistralai/Mistral-7B-Instruct-v0.3`
//!
//! # 使用示例
//! ```rust
//! use keycompute_vllm::VllmProvider;
//! use keycompute_provider_trait::ProviderAdapter;
//!
//! let provider = VllmProvider::new();
//! assert_eq!(provider.name(), "vllm");
//! // vLLM 支持 HuggingFace 格式的模型名
//! assert!(provider.supports_model("some-org/some-model"));
//! ```
//!
//! # vLLM 特点
//! - 高性能推理（PagedAttention、连续批处理）
//! - OpenAI API 兼容
//! - 支持流式输出
//! - 支持本地部署

pub mod adapter;

pub use adapter::{VLLM_COMMON_MODELS, VLLM_DEFAULT_ENDPOINT, VllmProvider};

// 复用 OpenAI 的协议类型，vLLM API 与 OpenAI API 完全兼容
pub use keycompute_openai::{OpenAIRequest, OpenAIResponse, OpenAIStreamResponse};
