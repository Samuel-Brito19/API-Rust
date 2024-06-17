use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct CreatePerson {
    pub nickname: String,
    pub name: String,
    pub birthdate: String,
    pub stack: Option<Vec<String>>,
}

#[derive(Deserialize, Serialize)]

pub struct Person {
    pub id: String,
    pub nickname: String,
    pub name: String,
    pub birthdate: String,
    pub stack: Option<Vec<String>>,
}

impl Person {
    pub fn from(row: &Row) -> Person {}
}
