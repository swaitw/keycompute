use client_api::error::Result;
use client_api::{
    AuthApi,
    api::auth::{
        AuthResponse, ForgotPasswordRequest, LoginRequest, MessageResponse, RefreshTokenRequest,
        RegisterRequest, ResetPasswordRequest,
    },
};

use super::api_client::get_client;

pub async fn login(email: &str, password: &str) -> Result<AuthResponse> {
    let client = get_client();
    let api = AuthApi::new(&client);
    api.login(&LoginRequest::new(email, password)).await
}

pub async fn register(email: &str, password: &str, name: Option<&str>) -> Result<AuthResponse> {
    let client = get_client();
    let api = AuthApi::new(&client);
    let mut req = RegisterRequest::new(email, password);
    if let Some(n) = name {
        req = req.with_name(n);
    }
    api.register(&req).await
}

pub async fn refresh_token(refresh_token: &str) -> Result<AuthResponse> {
    let client = get_client();
    AuthApi::new(&client)
        .refresh_token(&RefreshTokenRequest::new(refresh_token))
        .await
}

pub async fn forgot_password(email: &str) -> Result<MessageResponse> {
    let client = get_client();
    let api = AuthApi::new(&client);
    api.forgot_password(&ForgotPasswordRequest::new(email))
        .await
}

pub async fn reset_password(token: &str, new_password: &str) -> Result<MessageResponse> {
    let client = get_client();
    let api = AuthApi::new(&client);
    api.reset_password(&ResetPasswordRequest::new(token, new_password))
        .await
}

pub async fn verify_reset_token(token: &str) -> Result<MessageResponse> {
    let client = get_client();
    let api = AuthApi::new(&client);
    api.verify_reset_token(token).await
}

pub async fn verify_email(token: &str) -> Result<MessageResponse> {
    let client = get_client();
    let api = AuthApi::new(&client);
    api.verify_email(token).await
}
