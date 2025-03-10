use actix_web::web;
use sql_builder::{quote, SqlBuilder};
use std::{collections::HashSet, sync::Arc};

use deadpool_postgres::{GenericClient, Pool};
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;

pub type AsyncVoidResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;
pub type QueueEvent = (String, web::Json<CreatePerson>, Option<String>);
pub type AppQueue = deadqueue::unlimited::Queue<QueueEvent>;
#[derive(Deserialize)]
pub struct CreatePerson {
    pub nickname: String,
    pub nome: String,
    pub birthdate: String,
    pub stack: Option<Vec<String>>,
}

#[derive(Deserialize, Serialize)]
pub struct Person {
    pub id: String,
    pub nickname: String,
    pub nome: String,
    pub birthdate: String,
    pub stack: Option<Vec<String>>,
}

impl Person {
    pub fn from(row: &Row) -> Person {
        let stack: Option<String> = row.get(4);
        let stack = match stack {
            None => None,
            Some(s) => Some(s.split(' ').map(|s| s.to_string()).collect()),
        };
        Person {
            id: row.get(0),
            nickname: row.get(2),
            nome: row.get(3),
            birthdate: row.get(4),
            stack,
        }
    }
}

pub async fn db_count(conn: &deadpool_postgres::Client) -> Result<i64, Box<dyn std::error::Error>> {
    let rows = conn
        .query(
            "SELECT COUNT(1) FROM PEOPLE WHERE NICKNAME NOT LIKE 'WARMUP%';",
            &[],
        )
        .await?;
    let count: i64 = rows[0].get(0);
    Ok(count)
}

pub async fn db_search(
    conn: &deadpool_postgres::Client,
    target: String,
) -> Result<Vec<Person>, Box<dyn std::error::Error>> {
    let stmt = conn
        .prepare_cached(
            "
    SELECT ID, NICKNAME, NOME, BIRTHDATE, STACK
    FROM PEOPLE P
    WHERE P.BUSCA_TRGM LIKE $1
    LIMIT 50;
",
        )
        .await?;

    let rows = conn.query(&stmt, &[&target]).await?;
    let result = rows
        .iter()
        .map(|row| Person::from(row))
        .collect::<Vec<Person>>();
    Ok(result)
}

pub async fn db_get_person(
    conn: &deadpool_postgres::Client,
    id: &String,
) -> Result<Option<Person>, Box<dyn std::error::Error>> {
    let rows = conn
        .query(
            "
    SELECT ID, NICKNAME, NOME, BIRTHDATE, STACK FROM PEOPLE P
    WHERE P.ID=$1;",
            &[&id],
        )
        .await?;
    if rows.len() == 0 {
        return Ok(None);
    }
    Ok(Some(Person::from(&rows[0])))
}

#[derive(Deserialize)]
pub struct SearchParams {
    pub target: String,
}

pub async fn batch_insert(pool: Pool, queue: Arc<AppQueue>) {
    let mut nicknames = HashSet::<String>::new();
    let mut sql = String::new();
    while queue.len() > 0 {
        let (id, payload, stack) = queue.pop().await;
        if nicknames.contains(&payload.nickname) {
            continue;
        }
        nicknames.insert(payload.nickname.clone());
        let mut sql_builder = SqlBuilder::insert_into("PEOPLE");
        sql_builder
            .field("ID")
            .field("NICKNAME")
            .field("NOME")
            .field("BIRTHDATE")
            .field("STACK");
        sql_builder.values(&[
            &quote(id),
            &quote(&payload.nickname),
            &quote(&payload.nome),
            &quote(&payload.birthdate),
            &quote(stack.unwrap_or("".into())),
        ]);
        let mut this_sql = match sql_builder.sql() {
            Ok(x) => x,
            Err(_) => continue,
        };
        this_sql.pop();
        this_sql.push_str("ON CONFLICT DO NOTHING;");
        sql.push_str(&this_sql.as_str());
    }
    {
        let mut conn = match pool.get().await {
            Ok(x) => x,
            Err(_) => return,
        };
        let transaction = match conn.transaction().await {
            Ok(x) => x,
            Err(_) => return,
        };
        match transaction.batch_execute(&sql).await {
            Ok(_) => (),
            Err(_) => return,
        };
        let _ = transaction.commit().await;
    }
}
