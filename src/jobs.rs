use std::{sync::Arc, time::Duration};

use deadpool_postgres::Pool;

use crate::db::{batch_insert, AppQueue};

pub async fn db_clean_warmup(pool_async: Pool) {
    println!("cleaning warmup data...");
    tokio::time::sleep(Duration::from_secs(5)).await;
    pool_async
        .get()
        .await
        .unwrap()
        .execute("DELETE FROM PEOPLE WHERE NICKNAME LIKE 'WARMUP%';", &[])
        .await
        .unwrap();
}

pub async fn db_warmup() {
    println!("warming up...");
    tokio::time::sleep(Duration::from_secs(3)).await;
    let http_client = reqwest::Client::new();
    let nginx_url = "http://localhost:9999/people";
    let mount_body = |n: u16| {
        format!("{{\"nickname\":\"WARMUP::vaf{n}\",\"birthdate\":\"1999-01-01\",\"nome\":\"VAF\"}}")
    };
    let mut f = vec![];
    let v = vec![0, 1, 2, 1, 0];
    for i in 0..511 {
        for j in &v {
            f.push(
                http_client
                    .post(nginx_url)
                    .body(mount_body(j + i))
                    .header("Content-Type", "aplication/json")
                    .send(),
            );
        }
    }
    futures::future::join_all(f).await;
    println!("warmup finished");
}

pub async fn db_flush_queue(pool_async: Pool, queue_async: Arc<AppQueue>) {
    println!("queue flush started(loop every 2 seconds)");
    loop {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let queue = queue_async.clone();
        if queue.len() == 0 {
            continue;
        }
        batch_insert(pool_async.clone(), queue).await
    }
}
