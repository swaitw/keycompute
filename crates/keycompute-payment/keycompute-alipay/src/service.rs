//! 支付服务核心逻辑
//
//! 整合支付宝客户端和数据库操作，提供完整的支付流程

use crate::client::{AlipayClient, QueryResponse};
use crate::config::AlipayConfig;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

mod urlencoding {
    use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};

    pub fn encode(s: &str) -> String {
        utf8_percent_encode(s, NON_ALPHANUMERIC).to_string()
    }
}

/// 支付服务
pub struct PaymentService {
    client: AlipayClient,
    pool: sqlx::PgPool,
}

impl PaymentService {
    /// 创建新的支付服务
    pub fn new(config: AlipayConfig, pool: sqlx::PgPool) -> Result<Self, PaymentError> {
        let client = AlipayClient::new(config)?;
        Ok(Self { client, pool })
    }

    /// 从环境变量创建支付服务
    pub async fn from_env(pool: sqlx::PgPool) -> Result<Self, PaymentError> {
        let config = AlipayConfig::from_env()?;
        Self::new(config, pool)
    }

    /// 创建支付订单
    ///
    /// 返回支付URL，用于前端跳转到支付宝支付页面
    pub async fn create_order(
        &self,
        req: CreateOrderRequest,
    ) -> Result<CreateOrderResult, PaymentError> {
        // 生成商户订单号
        let out_trade_no = generate_out_trade_no();

        // 格式化金额
        let amount_str = format!("{:.2}", req.amount);

        // 生成支付URL
        let pay_url = self.client.page_pay_url(
            &out_trade_no,
            &amount_str,
            &req.subject,
            req.body.as_deref(),
        )?;

        // 创建数据库订单记录
        let db_req = keycompute_db::CreatePaymentOrderRequest {
            tenant_id: req.tenant_id,
            user_id: req.user_id,
            amount: req.amount,
            subject: req.subject.clone(),
            body: req.body.clone(),
            expire_minutes: self.client.config().timeout_minutes,
        };

        let order =
            keycompute_db::PaymentOrder::create(&self.pool, &db_req, &out_trade_no, &pay_url)
                .await
                .map_err(|e| PaymentError::DatabaseError(e.to_string()))?;

        Ok(CreateOrderResult {
            order_id: order.id,
            out_trade_no: order.out_trade_no,
            pay_url: order.pay_url.unwrap_or_default(),
            expired_at: order.expired_at,
        })
    }

    /// 创建手机网站支付订单
    pub async fn create_wap_order(
        &self,
        req: CreateOrderRequest,
    ) -> Result<CreateOrderResult, PaymentError> {
        let out_trade_no = generate_out_trade_no();
        let amount_str = format!("{:.2}", req.amount);

        let pay_url = self.client.wap_pay_url(
            &out_trade_no,
            &amount_str,
            &req.subject,
            req.body.as_deref(),
        )?;

        let db_req = keycompute_db::CreatePaymentOrderRequest {
            tenant_id: req.tenant_id,
            user_id: req.user_id,
            amount: req.amount,
            subject: req.subject.clone(),
            body: req.body.clone(),
            expire_minutes: self.client.config().timeout_minutes,
        };

        let order =
            keycompute_db::PaymentOrder::create(&self.pool, &db_req, &out_trade_no, &pay_url)
                .await
                .map_err(|e| PaymentError::DatabaseError(e.to_string()))?;

        Ok(CreateOrderResult {
            order_id: order.id,
            out_trade_no: order.out_trade_no,
            pay_url: order.pay_url.unwrap_or_default(),
            expired_at: order.expired_at,
        })
    }

    /// 创建扫码支付订单（当面付）
    ///
    /// 生成支付二维码，用户使用支付宝扫码完成支付
    /// 返回二维码内容和订单信息
    pub async fn create_qr_order(
        &self,
        req: CreateOrderRequest,
    ) -> Result<CreateQrOrderResult, PaymentError> {
        // 生成商户订单号
        let out_trade_no = generate_out_trade_no();

        // 格式化金额
        let amount_str = format!("{:.2}", req.amount);

        // 调用支付宝 precreate 接口
        let precreate_response = self
            .client
            .precreate(
                &out_trade_no,
                &amount_str,
                &req.subject,
                req.body.as_deref(),
            )
            .await
            .map_err(|e| PaymentError::ApiError(e.to_string()))?;

        if !precreate_response.is_success() {
            return Err(PaymentError::ApiError(
                precreate_response
                    .sub_msg
                    .unwrap_or_else(|| precreate_response.msg.clone()),
            ));
        }

        let qr_code = precreate_response.qr_code.clone().unwrap_or_default();

        // 创建数据库订单记录
        let db_req = keycompute_db::CreatePaymentOrderRequest {
            tenant_id: req.tenant_id,
            user_id: req.user_id,
            amount: req.amount,
            subject: req.subject.clone(),
            body: req.body.clone(),
            expire_minutes: self.client.config().timeout_minutes,
        };

        let order =
            keycompute_db::PaymentOrder::create(&self.pool, &db_req, &out_trade_no, &qr_code)
                .await
                .map_err(|e| PaymentError::DatabaseError(e.to_string()))?;

        Ok(CreateQrOrderResult {
            order_id: order.id,
            out_trade_no: order.out_trade_no,
            qr_code,
            expired_at: order.expired_at,
        })
    }

    /// 查询订单状态
    pub async fn query_order(&self, out_trade_no: &str) -> Result<QueryResponse, PaymentError> {
        self.client
            .query_order(out_trade_no)
            .await
            .map_err(|e| PaymentError::ApiError(e.to_string()))
    }

    /// 处理支付成功回调
    ///
    /// 验签成功后更新订单状态并充值用户余额
    pub async fn handle_notify(
        &self,
        params: HashMap<String, String>,
    ) -> Result<NotifyResult, PaymentError> {
        // 转换为参数列表
        let params_vec: Vec<(String, String)> = params.clone().into_iter().collect();

        // 验签
        if !self.client.verify_notify(&params_vec)? {
            return Err(PaymentError::InvalidSignature);
        }

        // 解析通知参数
        let out_trade_no = params
            .get("out_trade_no")
            .ok_or(PaymentError::MissingParam("out_trade_no"))?
            .clone();
        let trade_no = params
            .get("trade_no")
            .ok_or(PaymentError::MissingParam("trade_no"))?
            .clone();
        let trade_status = params
            .get("trade_status")
            .ok_or(PaymentError::MissingParam("trade_status"))?
            .clone();
        let total_amount: Decimal = params
            .get("total_amount")
            .ok_or(PaymentError::MissingParam("total_amount"))?
            .parse()
            .map_err(|_| PaymentError::InvalidAmount)?;

        // 查询订单
        let order = keycompute_db::PaymentOrder::find_by_out_trade_no(&self.pool, &out_trade_no)
            .await
            .map_err(|e| PaymentError::DatabaseError(e.to_string()))?
            .ok_or(PaymentError::OrderNotFound)?;

        // 检查订单状态
        if order.status != "pending" {
            // 订单已处理，返回成功避免重复通知
            return Ok(NotifyResult {
                order_id: order.id,
                status: order.status.clone(),
                amount: order.amount,
                trade_no: order.trade_no.unwrap_or_default(),
            });
        }

        // 检查交易状态
        if trade_status != "TRADE_SUCCESS" && trade_status != "TRADE_FINISHED" {
            // 交易失败，更新订单状态
            keycompute_db::PaymentOrder::mark_as_failed(&self.pool, order.id)
                .await
                .map_err(|e| PaymentError::DatabaseError(e.to_string()))?;

            return Ok(NotifyResult {
                order_id: order.id,
                status: "failed".to_string(),
                amount: order.amount,
                trade_no,
            });
        }

        // 开始事务处理支付成功
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| PaymentError::DatabaseError(e.to_string()))?;

        // 更新订单状态
        let notify_data = serde_json::to_value(&params)
            .map_err(|e| PaymentError::DatabaseError(e.to_string()))?;

        let updated_order = sqlx::query_as::<_, keycompute_db::PaymentOrder>(
            r#"
            UPDATE payment_orders
            SET status = 'paid',
                trade_no = $1,
                notify_data = $2,
                paid_at = NOW(),
                updated_at = NOW()
            WHERE id = $3
            RETURNING *
            "#,
        )
        .bind(&trade_no)
        .bind(sqlx::types::Json(&notify_data))
        .bind(order.id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| PaymentError::DatabaseError(e.to_string()))?;

        // 充值用户余额
        let description = format!("支付宝充值 - 订单号: {}", out_trade_no);
        keycompute_db::UserBalance::recharge(
            &mut tx,
            order.user_id,
            total_amount,
            Some(order.id),
            Some(&description),
        )
        .await
        .map_err(|e| PaymentError::DatabaseError(e.to_string()))?;

        // 提交事务
        tx.commit()
            .await
            .map_err(|e| PaymentError::DatabaseError(e.to_string()))?;

        Ok(NotifyResult {
            order_id: updated_order.id,
            status: "paid".to_string(),
            amount: updated_order.amount,
            trade_no,
        })
    }

    /// 主动同步订单状态
    ///
    /// 从支付宝查询订单状态并更新本地订单
    pub async fn sync_order_status(&self, out_trade_no: &str) -> Result<SyncResult, PaymentError> {
        // 查询本地订单
        let order = keycompute_db::PaymentOrder::find_by_out_trade_no(&self.pool, out_trade_no)
            .await
            .map_err(|e| PaymentError::DatabaseError(e.to_string()))?
            .ok_or(PaymentError::OrderNotFound)?;

        // 如果订单已处理，直接返回
        if order.status != "pending" {
            return Ok(SyncResult {
                order_id: order.id,
                status: order.status.clone(),
                changed: false,
            });
        }

        // 从支付宝查询订单状态
        let query_result = self
            .client
            .query_order(out_trade_no)
            .await
            .map_err(|e| PaymentError::ApiError(e.to_string()))?;

        if !query_result.is_success() {
            return Err(PaymentError::ApiError(
                query_result
                    .sub_msg
                    .unwrap_or_else(|| query_result.msg.clone()),
            ));
        }

        // 检查交易状态
        let trade_status = query_result.trade_status.as_deref().unwrap_or("");

        if trade_status == "TRADE_SUCCESS" || trade_status == "TRADE_FINISHED" {
            // 交易成功，更新订单并充值
            let trade_no = query_result.trade_no.clone().unwrap_or_default();
            let notify_data = serde_json::to_value(&query_result)
                .map_err(|e| PaymentError::DatabaseError(e.to_string()))?;

            let mut tx = self
                .pool
                .begin()
                .await
                .map_err(|e| PaymentError::DatabaseError(e.to_string()))?;

            let updated_order = sqlx::query_as::<_, keycompute_db::PaymentOrder>(
                r#"
                UPDATE payment_orders
                SET status = 'paid',
                    trade_no = $1,
                    notify_data = $2,
                    paid_at = NOW(),
                    updated_at = NOW()
                WHERE id = $3
                RETURNING *
                "#,
            )
            .bind(&trade_no)
            .bind(sqlx::types::Json(&notify_data))
            .bind(order.id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| PaymentError::DatabaseError(e.to_string()))?;

            // 充值余额
            let description = format!("支付宝充值(同步) - 订单号: {}", out_trade_no);
            keycompute_db::UserBalance::recharge(
                &mut tx,
                order.user_id,
                order.amount,
                Some(order.id),
                Some(&description),
            )
            .await
            .map_err(|e| PaymentError::DatabaseError(e.to_string()))?;

            tx.commit()
                .await
                .map_err(|e| PaymentError::DatabaseError(e.to_string()))?;

            Ok(SyncResult {
                order_id: updated_order.id,
                status: "paid".to_string(),
                changed: true,
            })
        } else if trade_status == "TRADE_CLOSED" {
            // 交易关闭
            let updated_order = keycompute_db::PaymentOrder::mark_as_failed(&self.pool, order.id)
                .await
                .map_err(|e| PaymentError::DatabaseError(e.to_string()))?;

            Ok(SyncResult {
                order_id: updated_order.id,
                status: "failed".to_string(),
                changed: true,
            })
        } else {
            // 等待付款或其他状态
            Ok(SyncResult {
                order_id: order.id,
                status: "pending".to_string(),
                changed: false,
            })
        }
    }

    /// 关闭订单
    pub async fn close_order(&self, out_trade_no: &str) -> Result<(), PaymentError> {
        // 调用支付宝关闭订单接口
        let result = self
            .client
            .close_order(out_trade_no)
            .await
            .map_err(|e| PaymentError::ApiError(e.to_string()))?;

        if !result.is_success() {
            return Err(PaymentError::ApiError(
                result.sub_msg.unwrap_or_else(|| result.msg.clone()),
            ));
        }

        // 更新本地订单状态
        keycompute_db::PaymentOrder::close(
            &self.pool,
            keycompute_db::PaymentOrder::find_by_out_trade_no(&self.pool, out_trade_no)
                .await
                .map_err(|e| PaymentError::DatabaseError(e.to_string()))?
                .ok_or(PaymentError::OrderNotFound)?
                .id,
        )
        .await
        .map_err(|e| PaymentError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// 获取用户余额
    pub async fn get_user_balance(&self, user_id: Uuid) -> Result<UserBalanceInfo, PaymentError> {
        let balance = keycompute_db::UserBalance::find_by_user(&self.pool, user_id)
            .await
            .map_err(|e| PaymentError::DatabaseError(e.to_string()))?
            .unwrap_or_else(|| keycompute_db::UserBalance {
                id: Uuid::nil(),
                tenant_id: Uuid::nil(),
                user_id,
                available_balance: Decimal::ZERO,
                frozen_balance: Decimal::ZERO,
                total_recharged: Decimal::ZERO,
                total_consumed: Decimal::ZERO,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            });

        Ok(UserBalanceInfo {
            user_id: balance.user_id,
            available_balance: balance.available_balance,
            frozen_balance: balance.frozen_balance,
            total_balance: balance.total_balance(),
            total_recharged: balance.total_recharged,
            total_consumed: balance.total_consumed,
        })
    }

    /// 获取支付客户端（用于需要直接调用支付宝API的场景）
    pub fn client(&self) -> &AlipayClient {
        &self.client
    }
}

/// 生成商户订单号
fn generate_out_trade_no() -> String {
    let timestamp = Utc::now().format("%Y%m%d%H%M%S");
    let random: String = (0..6)
        .map(|_| rand::random::<u8>() % 10)
        .map(|d| char::from_digit(d as u32, 10).unwrap())
        .collect();
    format!("KC{}{}", timestamp, random)
}

/// 创建订单请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateOrderRequest {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub amount: Decimal,
    pub subject: String,
    pub body: Option<String>,
}

/// 创建订单结果
#[derive(Debug, Clone, Serialize)]
pub struct CreateOrderResult {
    /// 订单ID
    pub order_id: Uuid,
    /// 商户订单号
    pub out_trade_no: String,
    /// 支付URL
    pub pay_url: String,
    /// 过期时间
    pub expired_at: DateTime<Utc>,
}

/// 回调处理结果
#[derive(Debug, Clone, Serialize)]
pub struct NotifyResult {
    pub order_id: Uuid,
    pub status: String,
    pub amount: Decimal,
    pub trade_no: String,
}

/// 同步订单结果
#[derive(Debug, Clone, Serialize)]
pub struct SyncResult {
    pub order_id: Uuid,
    pub status: String,
    pub changed: bool,
}

/// 用户余额信息
#[derive(Debug, Clone, Serialize)]
pub struct UserBalanceInfo {
    pub user_id: Uuid,
    pub available_balance: Decimal,
    pub frozen_balance: Decimal,
    pub total_balance: Decimal,
    pub total_recharged: Decimal,
    pub total_consumed: Decimal,
}

/// 创建扫码支付订单结果
#[derive(Debug, Clone, Serialize)]
pub struct CreateQrOrderResult {
    /// 订单ID
    pub order_id: Uuid,
    /// 商户订单号
    pub out_trade_no: String,
    /// 支付二维码内容（可用于生成二维码图片）
    pub qr_code: String,
    /// 过期时间
    pub expired_at: DateTime<Utc>,
}

impl CreateQrOrderResult {
    /// 获取二维码图片URL（使用第三方二维码生成服务）
    ///
    /// 示例：返回一个可以展示二维码图片的URL
    pub fn qr_code_image_url(&self) -> String {
        // 使用公共二维码生成API
        format!(
            "https://api.qrserver.com/v1/create-qr-code/?size=300x300&data={}",
            urlencoding::encode(&self.qr_code)
        )
    }
}

/// 支付错误
#[derive(Debug, thiserror::Error)]
pub enum PaymentError {
    #[error("配置错误: {0}")]
    ConfigError(String),
    #[error("API错误: {0}")]
    ApiError(String),
    #[error("数据库错误: {0}")]
    DatabaseError(String),
    #[error("签名验证失败")]
    InvalidSignature,
    #[error("缺少参数: {0}")]
    MissingParam(&'static str),
    #[error("订单不存在")]
    OrderNotFound,
    #[error("金额无效")]
    InvalidAmount,
    #[error("订单状态无效")]
    InvalidOrderStatus,
}

impl From<crate::config::ConfigError> for PaymentError {
    fn from(e: crate::config::ConfigError) -> Self {
        PaymentError::ConfigError(e.to_string())
    }
}

impl From<crate::client::ClientError> for PaymentError {
    fn from(e: crate::client::ClientError) -> Self {
        PaymentError::ApiError(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_out_trade_no() {
        let order_no = generate_out_trade_no();
        assert!(order_no.starts_with("KC"));
        assert_eq!(order_no.len(), 22); // KC + 14位时间戳 + 6位随机数
    }
}
