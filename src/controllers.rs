use std::sync::Arc;

use actix_web::{web, HttpResponse};
use chrono::NaiveDate;

use crate::{
    db::{AppQueue, CreatePerson},
    redis::{get_redis, set_redis},
};

use deadpool_postgres::Pool;

pub type APIResult = Result<HttpResponse, Box<dyn std::error::Error>>;

#[actix_web::post("/people")]

pub async fn create_person(
    redis_pool: web::Data<deadpool_redis::Pool>,
    payload: web::Json<CreatePerson>,
    queue: web::Json<Arc<AppQueue>>,
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
        .append_header(("Location", format!("/people/{id}")))
        .finish())
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
