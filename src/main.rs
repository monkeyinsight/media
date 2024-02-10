mod twitch;
mod youtube;

use dotenv::dotenv;
use actix_web::{
    put,get,post,delete,
    web,
    App,
    HttpServer,
    Responder,
    HttpResponse,
    Error,
    middleware,
    http::{header,StatusCode},
    dev,
    dev::{Service, Transform, ServiceRequest, ServiceResponse, forward_ready},
    body::EitherBody,
};
use actix_web::middleware::{ErrorHandlerResponse, ErrorHandlers};
use actix_multipart::Multipart;
use futures_util::StreamExt as _;
use serde::{Deserialize, Serialize};
use std::fs::{File,create_dir_all,OpenOptions};
use futures::future::{ok, LocalBoxFuture, Ready};
use walkdir::WalkDir;
use std::io::Write;

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

#[get("/files")]
async fn list_controller() -> impl Responder {
    let mut entries: Vec<String> = Vec::new();
    for e in WalkDir::new("./files/").into_iter().filter_map(|e| e.ok()) {
        if e.path().is_file() {
            entries.push(e.path().to_str().unwrap().to_owned().replace("./files/", "/"));
        }
    }

    entries.sort_by(|a,b| b.cmp(a));
    let mut html = String::new();
    html.push_str("<!doctype html><html><head><style>body{display:flex;flex-wrap:wrap;}img{margin:10px;max-width:300px}</style></head><body>");
    for entry in entries {
        html.push_str(&format!("<a href=\"/i{}\"><img src=\"/i{}\"/></a>", &entry, &entry))
    }
    html.push_str("</body></html>");
    return HttpResponse::Ok().body(html);
}

#[put("/files")]
pub async fn upload_controller(mut payload: Multipart) -> Result<HttpResponse, Error> {
    while let Some(item) = payload.next().await {
        let mut field = item.unwrap();
        let content_type = field.content_disposition();

        create_dir_all("./files/")?;
        let file_path = format!("./files/{}", content_type.get_filename().unwrap());
        let mut create_file = File::create(file_path).unwrap();

        while let Some(chunk) = field.next().await {
            let data = chunk.unwrap();
            create_file.write_all(&data).unwrap();
        }
    }

    Ok(HttpResponse::Ok().into())
}

fn add_error_header<B>(mut res: dev::ServiceResponse<B>) -> Result<ErrorHandlerResponse<B>, Error> {
    res.response_mut().headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("Error"),
    );

    Ok(ErrorHandlerResponse::Response(res.map_into_left_body()))
}

pub struct Authentication;

impl<S, B> Transform<S, ServiceRequest> for Authentication
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthenticationMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AuthenticationMiddleware { service })
    }
}

pub struct AuthenticationMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for AuthenticationMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let key = req.headers().get("Auth-token").unwrap().to_str().unwrap().to_owned();

        let mut authenticated: bool = match std::env::var("KEY") {
            Ok(k) => k == key,
            Err(_) => true,
        };

        if req.path().starts_with("/files") && *req.method() == "GET" {
            authenticated = true;
        }

        if !authenticated {
            let (request, _pl) = req.into_parts();
            let response = HttpResponse::Unauthorized()
                .json(Response { success: false, error: Some("not authorized".to_owned()) })
                .map_into_right_body();

            return Box::pin(async { Ok(ServiceResponse::new(request, response)) });
        }

        let res = self.service.call(req);

        Box::pin(async move { res.await.map(ServiceResponse::map_into_left_body) })
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
 
    let _ = OpenOptions::new().write(true)
        .create_new(true)
        .open("twitch.txt");

    let _ = OpenOptions::new().write(true)
        .create_new(true)
        .open("youtube.txt");

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .wrap(
                ErrorHandlers::new()
                    .handler(StatusCode::INTERNAL_SERVER_ERROR, add_error_header),
            )
            .wrap(Authentication)
            .service(list_controller)
            .service(upload_controller)
            .service(youtube_controller)
            .service(youtube_add_controller)
            .service(youtube_remove_controller)
            .service(twitch_controller)
            .service(twitch_add_controller)
            .service(twitch_remove_controller)
    }).bind(("127.0.0.1", 8888))?
        .run()
        .await
}
