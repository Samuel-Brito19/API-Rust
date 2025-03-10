use crate::db::*;
use crate::redis::*;
use actix_web::{web, HttpResponse};
use chrono::NaiveDate;
use deadpool_postgres::Pool;
use std::{sync::Arc, time::Duration};

use crate::{
    db::{AppQueue, CreatePerson, Person},
    redis::{get_redis, set_redis},
};

pub type APIResult = Result<HttpResponse, Box<dyn std::error::Error>>;

#[actix_web::post("/people")]

pub async fn create_person(
    redis_pool: web::Data<deadpool_redis::Pool>,
    payload: web::Json<CreatePerson>,
    queue: web::Data<Arc<AppQueue>>,
) -> APIResult {
    match validate_payload(&payload) {
        Some(response) => return Ok(response),
        None => (),
    };

    let redis_key = format!("a/{}", payload.nickname.clone());
    match get_redis(&redis_pool, &redis_key).await {
        Ok(_) => return Ok(HttpResponse::UnprocessableEntity().finish()),
        Err(_) => (),
    };
    let id = uuid::Uuid::new_v4().to_string();
    let dto = create_dto_and_queue(payload, &id, queue.clone());
    let body = serde_json::to_string(&dto)?;
    let _ = set_redis(&redis_pool, &id, &body).await;

    Ok(HttpResponse::Created()
        .append_header(("Location", format!("/pessoas/{id}")))
        .finish())
}

#[actix_web::get("people/{id}")]
pub async fn consult_person(
    id: web::Path<String>,
    pool: web::Data<Pool>,
    redis_pool: web::Data<deadpool_redis::Pool>,
) -> APIResult {
    let id = id.to_string();
    match get_redis(&redis_pool, &id).await {
        Err(_) => (),
        Ok(bytes) => return Ok(HttpResponse::Ok().body(bytes)),
    };
    let dto = db_get_person(&pool.get().await?, &id).await?;
    let body = serde_json::to_string(&dto)?;
    let body_async = body.clone();
    {
        let _ = set_redis(&redis_pool, &id, &body_async).await;
    }
    Ok(HttpResponse::Ok().body(body))
}

#[actix_web::get("/pessoas")]
pub async fn search_person(params: web::Query<SearchParams>, pool: web::Data<Pool>) -> APIResult {
    let t = format!("%{}%", params.target.to_lowercase());
    let result = db_search(&pool.get().await?, t).await?;
    let body = serde_json::to_string(&result)?;
    Ok(HttpResponse::Ok().body(body))
}

#[actix_web::get("/count-people")]
pub async fn count_people(pool: web::Data<Pool>) -> APIResult {
    tokio::time::sleep(Duration::from_secs(3)).await;
    let count: i64 = db_count(&pool.get().await?).await?;
    Ok(HttpResponse::Ok().body(count.to_string()))
}

// HELPER FUNCTIONS

fn validate_payload(payload: &CreatePerson) -> Option<HttpResponse> {
    if payload.nome.len() > 100 {
        return Some(HttpResponse::BadRequest().finish());
    }
    if payload.nickname.len() > 32 {
        return Some(HttpResponse::BadRequest().finish());
    }
    if NaiveDate::parse_from_str(&payload.birthdate, "%Y-%m-%d").is_err() {
        return Some(HttpResponse::BadRequest().finish());
    }
    if let Some(stack) = &payload.stack {
        for element in stack.clone() {
            if element.len() > 32 {
                return Some(HttpResponse::BadRequest().finish());
            }
        }
    }
    return None;
}

fn create_dto_and_queue(
    payload: web::Json<CreatePerson>,
    id: &String,
    queue: web::Data<Arc<AppQueue>>,
) -> Person {
    let stack = match &payload.stack {
        Some(v) => Some(v.join(" ")),
        None => None,
    };
    let nickname = payload.nickname.clone();
    let nome = payload.nome.clone();
    let birthdate = payload.birthdate.clone();
    let stack_vec = payload.stack.clone();
    let dto = Person {
        id: id.clone(),
        nickname,
        nome,
        birthdate,
        stack: stack_vec,
    };
    queue.push((id.clone(), payload, stack));
    return dto;
}
