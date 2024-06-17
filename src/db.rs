use serde::{Deserialize, Serialize};
use tokio_postgres::Row;

pub type AsyncVoidResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;
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
