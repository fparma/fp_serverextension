#[macro_use]
extern crate dotenv_codegen;

use arma_rs::{arma, Extension};
use chrono::prelude::{DateTime, Utc};
use env_logger::{Builder, Target};
use log::LevelFilter;
use mongodb::{
    bson::{doc, Document},
    options::ClientOptions,
    options::UpdateOptions,
    sync::{Database, Client}
};
use once_cell::sync::OnceCell;
use std::{env, thread, time::SystemTime};

static MONGODB: OnceCell<Database> = OnceCell::new();

#[arma]
fn init() -> Extension {
    let mut builder = Builder::from_default_env();
    builder.target(Target::Stdout);
    builder.filter_level(LevelFilter::Info);
    builder.init();

    Extension::build().command("log", log).finish()
}

fn connect() {
    if MONGODB.get().is_some() {
        log::warn!(target: "fp_extension", "Connection to DB already present!");
        return;
    }

    log::info!(target: "fp_extension", "Connecting to DB!");

    let _url = env::var("FP_EXTENSION_MONGO_DB_URL")
        .unwrap_or_else(|_| dotenv!("FP_EXTENSION_MONGO_DB_URL").to_string());
    let _db_name = env::var("FP_EXTENSION_MONGO_DB_DBNAME")
        .unwrap_or_else(|_| dotenv!("FP_EXTENSION_MONGO_DB_DBNAME").to_string());

    if let Ok(client_options) = ClientOptions::parse(_url) {
        // client_options.app_name = Some("FPArma Server Extension".to_string());
        if let Ok(client) = Client::with_options(client_options) {
            let _ = MONGODB.set(client.database(&_db_name[..]));
            log::info!(target: "fp_extension", "Connected to DB!");
        }
    }
}

pub fn log(id: String, log_level: i32, time: f64, message: String) -> String {
    let _id = id.clone();
    let _message = message.clone();
    thread::spawn(move || write_log(&_id, log_level, time, &_message));

    format!("{} {} {} {}", id, log_level, time, message)
}

fn write_log(id: &String, log_level: i32, time: f64, message: &String) {
    if MONGODB.get().is_none() {
        connect();
    }

    let _collection_name = env::var("FP_EXTENSION_MONGO_DB_COLLECTION")
        .unwrap_or_else(|_| dotenv!("FP_EXTENSION_MONGO_DB_COLLECTION").to_string());
    let _db = MONGODB.get().unwrap();

    let _dt: DateTime<Utc> = SystemTime::now().into();
    let _created_at: String = _dt.format("%FT%H:%M:%S%.3fZ").to_string();

    let _options = UpdateOptions::builder().upsert(true).build();
    let _collection = _db.collection::<Document>(&_collection_name[..]);

    _collection
        .update_one(
            doc! {"mission_id": id},
            doc! {
                    "$setOnInsert": doc! { "created_at": format!("{}", _created_at) },
                    "$push": doc! {"logs": doc! {"time": time, "level": log_level, "text": message}}
            },
            _options,
        )
        .unwrap();
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
        let (output, _) = unsafe {
            _extension.call(
                "log",
                Some(vec![
                    String::from(&_id),
                    log_level.to_string(),
                    time.to_string(),
                    String::from(&message),
                ]),
            )
        };
        assert_eq!(
            output,
            format!("{} {} {} {}", _id, log_level, time, message)
        );
    }
}
