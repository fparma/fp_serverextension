#[macro_use]
extern crate dotenv_codegen;

use std::env;
use futures::executor::block_on;
use arma_rs::{arma, Extension};
use mongodb::{Client, Database, options::ClientOptions};
use mongodb::bson::{doc, Document};
use mongodb::options::{UpdateOptions};
use chrono::prelude::{DateTime, Utc};
use std::time::SystemTime;
use once_cell::sync::OnceCell;

static MONGODB: OnceCell<Database> = OnceCell::new();

#[arma]
fn init() -> Extension {
    Extension::build()
        .command("log", log)
        .finish()
}

async fn connect() {
    if MONGODB.get().is_some() {
        println!("Connection to DB already present!");
        return;
    }

    println!("Connecting to DB!");

    let _url = env::var("EXTENSION_URL").unwrap_or_else(|_| dotenv!("EXTENSION_URL").to_string());
    let _db_name = env::var("EXTENSION_DBNAME").unwrap_or_else(|_| dotenv!("EXTENSION_DBNAME").to_string());

    if let Ok(client_options) = ClientOptions::parse(_url).await {
        // client_options.app_name = Some("FPArma Server Extension".to_string());
        if let Ok(client) = Client::with_options(client_options) {
            let _ = MONGODB.set(client.database(&_db_name[..]));
            println!("Connected to DB!");
        }
    }
}

pub fn log(id: String, log_level: i32, time: f64, message: String) -> String {
    // async {
    //     write_log(&id, log_level, time, &message).await
    // };
    let _future = write_log(&id, log_level, time, &message);
    block_on(_future); // <-- is blocking needed, cannot this be async?

    format!("{} {} {} {}", id, log_level, time, message)
}

async fn write_log(id: &String, log_level: i32, time: f64, message: &String) {
    if MONGODB.get().is_none() {
        connect().await;
    }

    let _collection_name = env::var("EXTENSION_COLLECTION").unwrap_or_else(|_| dotenv!("EXTENSION_COLLECTION").to_string());
    let _db = MONGODB.get().unwrap();

    let _dt: DateTime<Utc> = SystemTime::now().into();
    let _created_at: String = _dt.format("%FT%H:%M:%S%.3fZ").to_string();

    let _options = UpdateOptions::builder().upsert(true).build();
    let _collection = _db.collection::<Document>(&_collection_name[..]);

    _collection.update_one(
        doc! {"mission_id": id},
        doc! {
            "$setOnInsert": doc! { "created_at": format!("{}", _created_at) },
            "$push": doc! {"logs": doc! {"time": time, "level": log_level, "text": message}}
    }, _options).await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::init;
    use rand::Rng;

    #[test]
    fn logging() {
        let _extension = init().testing();
        let _id: String = "diwako///diwako_test///0.5///1337".to_string();
        let log_level: i32 = 2;
        let time: f64 = rand::thread_rng().gen_range(0.0..9999.0);
        let message: String = "This is a test message!".to_string();
        let (output, _) = unsafe { _extension.call("log", Some(vec![String::from(&_id), log_level.to_string(), time.to_string(), String::from(&message)])) };
        assert_eq!(output, format!("{} {} {} {}", _id, log_level, time, message));
    }
}
