#[macro_use]
extern crate dotenv_codegen;

use arma_rs::{arma, Extension};
use chrono::prelude::{DateTime, Utc};
use env_logger::Target;
use log::LevelFilter;
use mongodb::{
    bson::{doc, Document},
    options::{ClientOptions, UpdateOptions},
    sync::{Client, Database},
};
use once_cell::sync::OnceCell;
use std::{env, sync::RwLock, thread, time::SystemTime};

static MONGODB: OnceCell<RwLock<Option<Database>>> = OnceCell::new();

#[arma]
fn init() -> Extension {
    let mut builder = env_logger::Builder::from_default_env();
    builder.target(Target::Stdout);
    builder.filter_level(LevelFilter::Info);
    builder.init();
    // We unwrap as the cell should not be initialized
    MONGODB.set(RwLock::new(None)).unwrap();
    Extension::build().command("log", log).finish()
}

fn connect() -> Result<(), ()> {
    // We unwrap as the cell should be initialized by now.
    let lock = MONGODB.get().unwrap();

    {
        // In this scope we hold shared access to the lock.
        let read_l = lock.read().unwrap();
        if read_l.as_ref().is_some() {
            log::warn!(target: "fp_extension", "Connection to DB already present!");
            return Ok(());
        }
    }

    {
        // In this scope we hold exclusive access to the lock.
        let mut write_l = lock.write().unwrap();
        if write_l.as_mut().is_some() {
            Ok(())
        } else {
            log::info!(target: "fp_extension", "Connecting to DB!");

            let url = env::var("FP_EXTENSION_MONGO_DB_URL")
                .unwrap_or_else(|_| dotenv!("FP_EXTENSION_MONGO_DB_URL").to_string());
            let db_name = env::var("FP_EXTENSION_MONGO_DB_DBNAME")
                .unwrap_or_else(|_| dotenv!("FP_EXTENSION_MONGO_DB_DBNAME").to_string());

            match ClientOptions::parse(url) {
                Ok(client_options) => match Client::with_options(client_options) {
                    Ok(client) => {
                        let db = client.database(&db_name[..]);
                        // We unwrap here as we know the client should be empty as we check just that at the top of the function
                        // Else we had a concurrency issue and we should panic
                        write_l.replace(db);
                        log::info!(target: "fp_extension", "Connected to DB!");
                        Ok(())
                    }
                    Err(err) => {
                        log::error!(target: "fp_extension", "Error connecting to DB: {}", err);
                        Err(())
                    }
                },
                Err(err) => {
                    log::error!(target: "fp_extension", "Error parsing DB URL: {}", err);
                    Err(())
                }
            }
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
    match connect() {
        Ok(_) => {
            let db = MONGODB.get().unwrap().read().unwrap();
            let db = db.as_ref().unwrap();
            let dt: DateTime<Utc> = SystemTime::now().into();
            let created_at: String = dt.format("%FT%H:%M:%S%.3fZ").to_string();

            let options = UpdateOptions::builder().upsert(true).build();

            let collection_name = env::var("FP_EXTENSION_MONGO_DB_COLLECTION")
                .unwrap_or_else(|_| dotenv!("FP_EXTENSION_MONGO_DB_COLLECTION").to_string());

            let collection = db.collection::<Document>(&collection_name[..]);

            match collection.update_one(
                doc! {"mission_id": id},
                doc! {
                        "$setOnInsert": doc! { "created_at": format!("{}", created_at) },
                        "$push": doc! {"logs": doc! {"time": time, "level": log_level, "text": message}}
                },options,) {
            Ok(_) => (),
            Err(err) => log::error!(target: "fp_extension", "Error writing log: {}", err),
        }
        }
        Err(_) => {
            log::error!(target: "fp_extension", "Failed to establish DB connection");
        }
    };
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
