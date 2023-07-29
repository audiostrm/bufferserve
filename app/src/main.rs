mod types;
mod util;

use actix_cors::Cors;
use actix_web::{get, http, web, App, HttpResponse, HttpServer, Responder};
use reqwest::Client;
use sqlx::{Pool, Postgres};
use std::env;
use types::Audio;
use util::create_db_conn_pool;

type DbPool = Pool<Postgres>;

#[cfg(debug_assertions)]
use dotenvy::dotenv;

#[get("/api/{id}")]
async fn query(id: web::Path<String>, pool: web::Data<DbPool>) -> impl Responder {
    let query = sqlx::query_as!(
        Audio,
        "SELECT audio, id FROM \"Audio\" WHERE id = $1",
        id.into_inner()
    )
    .fetch_one(pool.as_ref())
    .await;

    match query {
        Ok(row) => {
            let client = Client::new();
            let url = row.audio;
            let response = client.get(&url).send().await;

            match response {
                Ok(body) => {
                    let audio_bytes = body.bytes().await;

                    match audio_bytes {
                        Ok(byte) => HttpResponse::Ok().content_type("audio/mpeg").body(byte),
                        Err(_) => HttpResponse::InternalServerError()
                            .content_type("application/json")
                            .body(format!(r#"{{"error":"Error occured while buffering"}}"#)),
                    }
                }
                Err(_) => HttpResponse::InternalServerError()
                    .content_type("application/json")
                    .body(format!(
                        r#"{{"error":"Error occured while fetching audio"}}"#
                    )),
            }
        }
        Err(_) => HttpResponse::BadRequest()
            .content_type("application/json")
            .body(format!(r#"{{"error":"Audio not found"}}"#)),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    #[cfg(debug_assertions)]
    dotenv().ok();

    let port = env::var("PORT").expect("Missing port number");
    let port = port.parse::<u16>().expect("Invalid port given");
    let db_conn_url = env::var("DATABASE_URL").expect("Missing DATABASE_URL");
    let db_conn_pool = create_db_conn_pool(&db_conn_url, 5).await;

    println!("[CONNECTION] Connected Database");

    HttpServer::new(move || {
        // let cors = Cors::default()
        //     .allowed_origin("https://www.audiostream.space")
        //     .allowed_origin("http://localhost:6006")
        //     .allowed_methods(vec!["GET"])
        //     .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT])
        //     .allowed_header(http::header::CONTENT_TYPE)
        //     .max_age(3600);
        let cors = Cors::permissive();

        App::new()
            .wrap(cors)
            .app_data(web::Data::new(db_conn_pool.clone()))
            .service(query)
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
