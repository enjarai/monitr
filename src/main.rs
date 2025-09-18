use std::env;

use actix_web::{get, http::header::ContentType, post, web::{Json, Query}, App, HttpRequest, HttpResponse, HttpServer, Responder};
use anyhow::{anyhow, Context as _, Result};
use chrono::{DateTime, Local};
use env_logger::Target;
use log::{error, info, LevelFilter};
use once_cell::sync::Lazy;
use prometheus::{register_int_gauge, Encoder as _, IntGauge, TextEncoder};
use reqwest::Client;
use serde::{Deserialize, Serialize};

static HEARTRATE_GAUGE: Lazy<IntGauge> =
    Lazy::new(|| register_int_gauge!("bang_heartrate", "The amount of clients listening for clicks").unwrap());

struct Token(String);
struct TrainToken(String);

#[derive(Deserialize)]
struct TrainQuery {
    current_time_string: String,
    from: String,
    to: String,
}

#[derive(Deserialize)]
struct TrainTrips {
    trips: Vec<Trip>,
}

#[derive(Deserialize)]
struct Trip {
    legs: Vec<Leg>,
}

#[derive(Deserialize)]
struct Leg {
    origin: LegStation,
}

#[allow(non_snake_case)]
#[derive(Deserialize, Serialize)]
struct LegStation {
    stationCode: String,
    plannedDateTime: String,
    actualDateTime: String,
    plannedTrack: String,
    actualTrack: String,
}

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
    let train_token = env::var("TRAIN_TOKEN")?;

    let server = HttpServer::new(move || {
        App::new()
            .app_data(Token(token.clone()))
            .app_data(TrainToken(train_token.clone()))
            .service(stats)
            .service(metrics)
            .service(trains)
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

#[get("/trains")]
async fn trains(req: HttpRequest, query: Query<TrainQuery>) -> impl Responder {
    let token = &req.app_data::<Token>().unwrap().0;
    let train_token = &req.app_data::<TrainToken>();
    if let Some(value) = req.headers().get("Authorization") 
            && let Ok(header) = value.to_str()
            && header.starts_with("Bearer ")
            && header["Bearer ".len()..header.len()] == *token
            && let Some(train_token) = train_token {
        let url = format!(
            "https://gateway.apiportal.ns.nl/reisinformatie-api/api/v3/trips?dateTime={}&fromStation={}&toStation={}", 
            query.current_time_string, query.from, query.to
        );
        
        match fetch_trains(&url, train_token).await {
            Ok(r) => r,
            Err(e) => HttpResponse::InternalServerError()
                .body(format!("{}", e)),
        }
    } else {
        HttpResponse::NotFound()
            .finish()
    }
}

async fn fetch_trains(url: &str, train_token: &TrainToken) -> Result<HttpResponse> {
    let client = Client::new();
    let res = client.get(url)
        .header("Ocp-Apim-Subscription-Key", train_token.0.clone())
        .send()
        .await?;
    let json = res.json::<TrainTrips>().await?;

    let now = Local::now();
    let trip = json.trips.iter().find(|t| match t.legs.get(0) {
        Some(l) => {
            let str = &l.origin.actualDateTime;
            match DateTime::parse_from_rfc3339(&format!("{}:00", &str[..str.len() - 2])) {
            Ok(t) => t > now,
            Err(e) => todo!("{}", e),
        }},
        None => todo!("test"),
    });

    Ok(match trip {
        Some(t) => HttpResponse::Ok()
            .append_header(ContentType::plaintext())
            .json(&t
                .legs
                .get(0)
                .ok_or(anyhow!("No legs?"))?
                .origin),
        None => HttpResponse::NotFound()
            .finish(),
    })
}
