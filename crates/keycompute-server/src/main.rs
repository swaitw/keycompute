//! KeyCompute 后端服务主入口
//!
//! 这是整个 KeyCompute 系统的可执行入口，负责：
//! 1. 加载配置（环境变量 + 配置文件）
//! 2. 初始化可观测性（日志、指标、追踪）
//! 3. 建立数据库连接并运行迁移
//! 4. 初始化所有业务模块（Auth、RateLimit、Pricing、Routing、Gateway、Billing 等）
//! 5. 启动 HTTP 服务器

use keycompute_config::AppConfig;
use keycompute_db::{DatabaseConfig as DbConfig, DatabaseManager};
use keycompute_observability::{init_dev_observability, init_observability};
use keycompute_server::{AppState, AppStateConfig, init_global_crypto, run};
use std::sync::Arc;
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ==================== 阶段 1: 加载配置 ====================
    info!("KeyCompute 启动中...");

    let config = match AppConfig::load() {
        Ok(cfg) => {
            info!("配置加载成功");
            cfg
        }
        Err(e) => {
            eprintln!("配置加载失败: {}", e);
            std::process::exit(1);
        }
    };

    // 验证配置
    if let Err(e) = config.validate() {
        eprintln!("配置验证失败: {}", e);
        std::process::exit(1);
    }

    // ==================== 阶段 2: 初始化可观测性 ====================
    // 根据环境选择日志格式
    let env = std::env::var("KC__ENV").unwrap_or_else(|_| "production".to_string());
    if env == "development" || env == "dev" {
        init_dev_observability();
        info!("开发环境可观测性已初始化");
    } else {
        init_observability();
        info!("生产环境可观测性已初始化");
    }

    // ==================== 阶段 3: 初始化全局加密 ====================
    if let Err(e) = init_global_crypto(&config) {
        error!("全局加密初始化失败: {}", e);
        std::process::exit(1);
    }

    // ==================== 阶段 4: 建立数据库连接 ====================
    info!("正在连接数据库...");

    // 转换配置类型
    let db_config = DbConfig {
        url: config.database.url.clone(),
        max_connections: config.database.max_connections,
        min_connections: config.database.min_connections,
        connect_timeout: config.database.connect_timeout_secs,
        idle_timeout: config.database.idle_timeout_secs,
        max_lifetime: config.database.max_lifetime_secs,
    };

    let db_manager = match DatabaseManager::new(&db_config).await {
        Ok(manager) => {
            info!("数据库连接成功");
            manager
        }
        Err(e) => {
            error!("数据库连接失败: {}", e);
            std::process::exit(1);
        }
    };

    // 测试数据库连接
    if let Err(e) = db_manager.test_connection().await {
        error!("数据库连接测试失败: {}", e);
        std::process::exit(1);
    }

    // 运行数据库迁移
    info!("正在运行数据库迁移...");
    if let Err(e) = db_manager.migrate().await {
        error!("数据库迁移失败: {}", e);
        std::process::exit(1);
    }
    info!("数据库迁移完成");

    let pool = Arc::new(db_manager.pool().clone());

    // ==================== 阶段 5: 初始化应用状态 ====================
    info!("正在初始化应用状态...");

    let state_config = AppStateConfig::from_config(&config);
    let app_state = AppState::with_pool_and_config(pool, state_config);

    // 验证生产环境配置
    if env != "development" && env != "dev" {
        if let Err(e) = app_state.validate_for_production() {
            error!("生产环境验证失败: {}", e);
            std::process::exit(1);
        }
    }

    info!("应用状态初始化完成");

    // ==================== 阶段 6: 启动服务器 ====================
    info!("准备启动服务器...");

    let server_config = config.server.clone();

    // 优雅关闭处理
    let shutdown = setup_shutdown_handler();

    info!(
        "KeyCompute 服务器即将启动于 {}:{}",
        server_config.bind_addr, server_config.port
    );

    // 启动服务器（带优雅关闭支持）
    tokio::select! {
        result = run(server_config, app_state) => {
            if let Err(e) = result {
                error!("服务器运行错误: {}", e);
                std::process::exit(1);
            }
        }
        _ = shutdown => {
            info!("收到关闭信号，正在优雅关闭...");
        }
    }

    info!("KeyCompute 服务器已停止");
    Ok(())
}

/// 设置优雅关闭信号处理器
///
/// 监听 SIGINT (Ctrl+C) 和 SIGTERM 信号
fn setup_shutdown_handler() -> tokio::sync::oneshot::Receiver<()> {
    let (tx, rx) = tokio::sync::oneshot::channel();

    tokio::spawn(async move {
        let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
            .expect("Failed to create SIGINT handler");
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to create SIGTERM handler");

        tokio::select! {
            _ = sigint.recv() => {
                info!("收到 SIGINT 信号");
            }
            _ = sigterm.recv() => {
                info!("收到 SIGTERM 信号");
            }
        }

        let _ = tx.send(());
    });

    rx
}
