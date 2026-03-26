//! 支付宝签名和验签模块
//
//! 使用RSA2 (SHA256withRSA) 签名算法

use base64::{Engine, engine::general_purpose::STANDARD};
use rsa::{
    RsaPrivateKey, RsaPublicKey,
    pkcs1::{DecodeRsaPrivateKey, DecodeRsaPublicKey},
    pkcs1v15::{Signature, SigningKey, VerifyingKey},
    pkcs8::{DecodePrivateKey, DecodePublicKey},
    signature::{SignatureEncoding, Signer, Verifier},
};
use sha2::Sha256;

/// 签名器
pub struct AlipaySigner {
    private_key: RsaPrivateKey,
}

impl AlipaySigner {
    /// 从PEM格式私钥创建签名器
    pub fn from_pem(pem: &str) -> Result<Self, SignError> {
        // 先尝试 PKCS#8 格式
        let private_key = if let Ok(key) = RsaPrivateKey::from_pkcs8_pem(pem) {
            key
        } else if let Ok(key) = RsaPrivateKey::from_pkcs1_pem(pem) {
            // 再尝试 PKCS#1 格式
            key
        } else {
            return Err(SignError::InvalidPrivateKey);
        };

        Ok(Self { private_key })
    }

    /// 对字符串进行签名
    pub fn sign(&self, content: &str) -> Result<String, SignError> {
        let signing_key = SigningKey::<Sha256>::new_unprefixed(self.private_key.clone());
        let signature = signing_key.sign(content.as_bytes());

        // 转换为Base64
        let sig_bytes = signature.to_bytes();
        Ok(STANDARD.encode(sig_bytes.as_ref()))
    }
}

/// 验签器
pub struct AlipayVerifier {
    public_key: RsaPublicKey,
}

impl AlipayVerifier {
    /// 从PEM格式公钥创建验签器
    pub fn from_pem(pem: &str) -> Result<Self, SignError> {
        // 先尝试 PKCS#8 格式
        let public_key = if let Ok(key) = RsaPublicKey::from_public_key_pem(pem) {
            key
        } else if let Ok(key) = RsaPublicKey::from_pkcs1_pem(pem) {
            key
        } else {
            return Err(SignError::InvalidPublicKey);
        };

        Ok(Self { public_key })
    }

    /// 验证签名
    pub fn verify(&self, content: &str, signature: &str) -> Result<bool, SignError> {
        let sig_bytes = STANDARD
            .decode(signature)
            .map_err(|_| SignError::InvalidSignature)?;

        let signature =
            Signature::try_from(sig_bytes.as_slice()).map_err(|_| SignError::InvalidSignature)?;

        let verifying_key = VerifyingKey::<Sha256>::new_unprefixed(self.public_key.clone());
        match verifying_key.verify(content.as_bytes(), &signature) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

/// 签名错误
#[derive(Debug, thiserror::Error)]
pub enum SignError {
    #[error("无效的私钥")]
    InvalidPrivateKey,
    #[error("无效的公钥")]
    InvalidPublicKey,
    #[error("无效的签名")]
    InvalidSignature,
    #[error("签名失败: {0}")]
    SignFailed(String),
    #[error("验签失败: {0}")]
    VerifyFailed(String),
}

/// 对参数进行签名
//
/// 按照支付宝规范，需要先对参数按key排序，然后拼接成待签名字符串
pub fn sign_params(
    params: &[(String, String)],
    signer: &AlipaySigner,
) -> Result<String, SignError> {
    // 过滤空值和sign字段
    let mut filtered: Vec<_> = params
        .iter()
        .filter(|(k, v)| !v.is_empty() && k != "sign")
        .collect();

    // 按key排序
    filtered.sort_by(|a, b| a.0.cmp(&b.0));

    // 拼接成待签名字符串 (key1=value1&key2=value2)
    let content = filtered
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("&");

    signer.sign(&content)
}

/// 验证参数签名
//
/// 从参数中提取sign字段，验证其他参数的签名
pub fn verify_params(
    params: &[(String, String)],
    signature: &str,
    verifier: &AlipayVerifier,
) -> Result<bool, SignError> {
    // 过滤空值和sign字段
    let mut filtered: Vec<_> = params
        .iter()
        .filter(|(k, v)| !v.is_empty() && k != "sign" && k != "sign_type")
        .collect();

    // 按key排序
    filtered.sort_by(|a, b| a.0.cmp(&b.0));

    // 拼接成待验签字符串
    let content = filtered
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("&");

    verifier.verify(&content, signature)
}

#[cfg(test)]
mod tests {
    use super::*;

    // 注意：实际测试需要真实的RSA密钥对
    // 这里只测试基本的结构

    #[test]
    fn test_sign_params_ordering() {
        let params = vec![
            ("c".to_string(), "3".to_string()),
            ("a".to_string(), "1".to_string()),
            ("b".to_string(), "2".to_string()),
        ];

        let mut filtered: Vec<_> = params.iter().filter(|(k, _)| !k.is_empty()).collect();
        filtered.sort_by(|a, b| a.0.cmp(&b.0));

        let content = filtered
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        assert_eq!(content, "a=1&b=2&c=3");
    }
}
