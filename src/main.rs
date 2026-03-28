mod cache;
mod config;
mod cors;
mod crypto;
mod pipe;
mod proxy;
mod templates;

use actix_web::{middleware::Compress, web, App, HttpServer};
use config::PORT;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();

    let port = *PORT;
    let workers = num_cpus::get();
    eprintln!("Proxy started on http://0.0.0.0:{}", port);
    eprintln!("   Workers: {}", workers);
    eprintln!(
        "   CORS: {}",
        if *config::ENABLE_CORS {
            format!("enabled (origins: {:?})", *config::ALLOWED_ORIGINS)
        } else {
            "disabled (allow all)".to_string()
        }
    );

    HttpServer::new(|| {
        App::new()
            .wrap(Compress::default())
            .wrap(actix_web::middleware::DefaultHeaders::new().add(("Vary", "Accept-Encoding")))
            .route("/", web::get().to(proxy::proxy_handler))
            .route(
                "/",
                web::method(actix_web::http::Method::OPTIONS).to(cors::handle_options),
            )
            .route("/health", web::get().to(|| async { "OK" }))
    })
    .workers(workers)
    .backlog(4096)
    .max_connections(25_000)
    .keep_alive(actix_web::http::KeepAlive::Timeout(
        std::time::Duration::from_secs(90),
    ))
    .client_request_timeout(std::time::Duration::from_secs(20))
    .bind(format!("0.0.0.0:{}", port))?
    .run()
    .await
}
