use std::env;

use actix_web::{get, http::header::ContentType, post, web::Json, App, HttpRequest, HttpResponse, HttpServer, Responder};
use anyhow::{anyhow, Context as _, Result};
use env_logger::Target;
use log::{error, info, LevelFilter};
use once_cell::sync::Lazy;
use prometheus::{register_int_gauge, Encoder as _, IntGauge, TextEncoder};
use serde::Deserialize;

static HEARTRATE_GAUGE: Lazy<IntGauge> =
    Lazy::new(|| register_int_gauge!("bang_heartrate", "The amount of clients listening for clicks").unwrap());

struct Token(String);

#[derive(Deserialize)]
struct Stats {
    heartrate: i64,
}

#[actix_web::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_default_env()
        .target(Target::Stdout)
        .filter_level(LevelFilter::Info)
        .init();

    let addr = env::var("ADDRESS")?;
    let port = env::var("PORT")?.parse::<u16>()?;
    let token = env::var("TOKEN")?;

    let server = HttpServer::new(move || {
        App::new()
            .app_data(Token(token.clone()))
            .service(stats)
            .service(metrics)
    })
    .bind((addr, port))
    .context("Failed to bind address")?;

    info!("Server configured, running...");
    server
        .run()
        .await
        .map_err(|e| anyhow!("Failed to run server: {}", e))


}

#[post("/stats")]
async fn stats(req: HttpRequest, stats: Json<Stats>) -> impl Responder {
    let token = &req.app_data::<Token>().unwrap().0;
    if let Some(value) = req.headers().get("Authorization") 
            && let Ok(header) = value.to_str()
            && header.starts_with("Bearer ")
            && header["Bearer ".len()..header.len()] == *token {

        HEARTRATE_GAUGE.set(stats.heartrate);
        HttpResponse::Ok()
    } else {
        HttpResponse::NotFound()
    }
}


#[get("/metrics")]
async fn metrics(_req: HttpRequest) -> impl Responder {    
    let mut buffer = Vec::new();
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    if let Err(err) = encoder.encode(&metric_families, &mut buffer) {
        error!("Error providing metrics: {}", err);
        return HttpResponse::InternalServerError().finish();
    }
    HttpResponse::Ok()
        .append_header(ContentType::plaintext())
        .body(buffer)
}
