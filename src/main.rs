mod controllers;
use controllers::*;

mod db;
use db::*;

mod jobs;

mod redis;
use jobs::*;

use actix_web::{http::KeepAlive, web, App, HttpServer};
use deadpool_postgres::{Config, PoolConfig, Runtime, Timeouts};
use deadpool_redis::{ConnectionAddr, ConnectionInfo, RedisConnectionInfo};
use std::env;
use std::{sync::Arc, time::Duration};
use tokio_postgres::NoTls;

#[tokio::main]
async fn main() -> AsyncVoidResult {
    let mut cfg = Config::new();
    cfg.host = Some(
        env::var("DB_HOST")
            .unwrap_or("localhost".into())
            .to_string(),
    );
    cfg.port = Some(5432);
    cfg.dbname = Some("rinhadb".to_string());
    cfg.user = Some("root".to_string());
    cfg.password = Some("1234".to_string());

    let pool_size = env::var("POOL_SIZE")
        .unwrap_or("125".to_string())
        .parse::<usize>()
        .unwrap();

    cfg.pool = PoolConfig::new(pool_size).into();
    println!("Creating postgres pool...");
    let pool = cfg.create_pool(Some(Runtime::Tokio1), NoTls)?;
    println!("postgres pool succesfully created");

    let mut cfg = deadpool_redis::Config::default();
    let redis_host = env::var("REDIS_HOST").unwrap_or("0.0.0.0".into());
    cfg.connection = Some(ConnectionInfo {
        addr: ConnectionAddr::Tcp(redis_host, 6379),
        redis: RedisConnectionInfo {
            db: 0,
            username: None,
            password: None,
        },
    });
    cfg.pool = Some(PoolConfig {
        max_size: 995,
        timeouts: Timeouts {
            wait: Some(Duration::from_secs(60)),
            create: Some(Duration::from_secs(60)),
            recycle: Some(Duration::from_secs(60)),
        },
    });
    println!("creating redis pool...");
    let redis_pool = cfg.create_pool(Some(Runtime::Tokio1))?;
    println!("redis pool succesfully created");

    tokio::spawn(async move { db_warmup().await });

    let pool_async = pool.clone();
    tokio::spawn(async move { db_clean_warmup(pool_async).await });
    let pool_async = pool.clone();
    let queue = Arc::new(AppQueue::new());
    let queue_async = queue.clone();
    tokio::spawn(async move { db_flush_queue(pool_async, queue_async).await });

    let http_port = env::var("HTTP_PORT").unwrap_or("80".into());

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(redis_pool.clone()))
            .app_data(web::Data::new(queue.clone()))
            .service(create_person)
            .service(consult_person)
            .service(search_person)
            .service(count_people)
    })
    .keep_alive(KeepAlive::Os)
    .bind(("127.0.0.1", 8080))?
    .run()
    .await?;

    Ok(())
}
