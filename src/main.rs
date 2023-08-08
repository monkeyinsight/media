mod twitch;
mod youtube;

use actix_web::{get,post,delete,web,App,HttpServer,Responder,HttpResponse};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;

#[derive(Serialize, Deserialize)]
struct ChannelObj {
    channel: String
}

#[derive(Serialize, Deserialize)]
struct Response {
    success: bool,
    error: Option<String>
}

#[get("/youtube")]
async fn youtube_controller() -> impl Responder {
    let youtube = youtube::get_subscriptions().await;

    let mut handles = Vec::new();

    for sub in youtube {
        let job = tokio::spawn(youtube::get_videos(sub));
        handles.push(job);
    }

    let mut results = Vec::new();

    for job in handles {
        let channel = job.await.unwrap();
        match channel {
            Ok(ch) => results.push(ch),
            Err(_e) => {}
        }
    }

    return serde_json::to_string(&results);
}

#[post("/youtube")]
async fn youtube_add_controller(form: web::Json<ChannelObj>) -> HttpResponse {
    match youtube::add(&form.channel).await {
        Ok(_) => HttpResponse::Ok().json(Response { success: true, error: None }),
        Err(e) => HttpResponse::InternalServerError().json(Response { success: false, error: Some(e.to_owned()) })
    }
}

#[delete("/youtube")]
async fn youtube_remove_controller(form: web::Json<ChannelObj>) -> HttpResponse {
    match youtube::remove(&form.channel).await {
        Ok(_) => HttpResponse::Ok().json(Response { success: true, error: None }),
        Err(e) => HttpResponse::InternalServerError().json(Response { success: false, error: Some(e.to_owned()) })
    }
}

#[get("/twitch")]
async fn twitch_controller() -> impl Responder {
    let twitch = twitch::get_subscriptions().await;

    let mut handles = Vec::new();

    for sub in twitch {
        let job = tokio::spawn(twitch::get_status(sub));
        handles.push(job);
    }

    let mut results = Vec::new();

    for job in handles {
        let channel = job.await.unwrap();
        match channel {
            Ok(ch) => results.push(ch),
            Err(_e) => {}
        }
    }

    return serde_json::to_string(&results);
}

#[post("/twitch")]
async fn twitch_add_controller(form: web::Json<ChannelObj>) -> HttpResponse {
    match twitch::add(&form.channel).await {
        Ok(_) => HttpResponse::Ok().json(Response { success: true, error: None }),
        Err(e) => HttpResponse::InternalServerError().json(Response { success: false, error: Some(e.to_owned()) })
    }
}

#[delete("/twitch")]
async fn twitch_remove_controller(form: web::Json<ChannelObj>) -> HttpResponse {
    match twitch::remove(&form.channel).await {
        Ok(_) => HttpResponse::Ok().json(Response { success: true, error: None }),
        Err(e) => HttpResponse::InternalServerError().json(Response { success: false, error: Some(e.to_owned()) })
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let _ = OpenOptions::new().write(true)
        .create_new(true)
        .open("twitch.txt");

    let _ = OpenOptions::new().write(true)
        .create_new(true)
        .open("youtube.txt");

    HttpServer::new(|| App::new()
        .service(youtube_controller)
        .service(youtube_add_controller)
        .service(youtube_remove_controller)
        .service(twitch_controller)
        .service(twitch_add_controller)
        .service(twitch_remove_controller)
    )
        .bind(("127.0.0.1", 8888))?
        .run()
        .await
}