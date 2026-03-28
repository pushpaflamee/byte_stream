use actix_web::{web::Bytes, HttpRequest, HttpResponse, Responder};
use futures_util::StreamExt;
use once_cell::sync::Lazy;
use reqwest::Client;
use std::collections::HashMap;
use tokio::task;
use url::Url;

use crate::cache::{CachedAsset, ASSET_CACHE, M3U8_CACHE};
use crate::config::ENABLE_CORS;
use crate::cors::{get_valid_origin, set_cors_headers};
use crate::crypto::decrypt_url;
use crate::pipe::process_pipe_body;
use crate::templates::generate_headers_for_url;

static CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .pool_idle_timeout(std::time::Duration::from_secs(90))
        .pool_max_idle_per_host(30)
        .tcp_keepalive(std::time::Duration::from_secs(60))
        .tcp_nodelay(true)
        .danger_accept_invalid_certs(true)
        .redirect(reqwest::redirect::Policy::limited(10))
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(45))
        .build()
        .expect("Failed to build reqwest client")
});

const FORWARDED_HEADERS: &[&str] = &[
    "content-type",
    "content-length",
    "content-range",
    "accept-ranges",
    "cache-control",
    "expires",
    "last-modified",
    "etag",
    "content-encoding",
    "vary",
];

pub async fn proxy_handler(req: HttpRequest) -> impl Responder {
    let query_future = task::spawn_blocking({
        let query_string = req.query_string().to_string();
        move || {
            query_string
                .split('&')
                .filter_map(|s| {
                    let mut split = s.splitn(2, '=');
                    let key = split.next()?;
                    let value = split.next().unwrap_or("");
                    Some((
                        key.to_string(),
                        urlencoding::decode(value)
                            .map(|v| v.into_owned())
                            .unwrap_or_else(|_| value.to_string()),
                    ))
                })
                .collect::<HashMap<String, String>>()
        }
    });

    let query = match query_future.await {
        Ok(q) => q,
        Err(_) => return HttpResponse::InternalServerError().body("Query parsing failed"),
    };

    // Bail out if origin is not explicitly allowed :sob:
    let acao = get_valid_origin(&req);
    if *ENABLE_CORS && acao.is_none() {
        return HttpResponse::Forbidden().finish();
    }

    let target_url = if let Some(encrypted) = query.get("u") {
        match decrypt_url(encrypted) {
            Some(url) => url,
            None => return HttpResponse::BadRequest().body("Invalid encrypted URL"),
        }
    } else if let Some(url) = query.get("url") {
        url.clone()
    } else {
        return HttpResponse::BadRequest().body("Missing u parameter");
    };

    let target_url_parsed = match Url::parse(&target_url) {
        Ok(u) => u,
        Err(e) => return HttpResponse::BadRequest().body(format!("Invalid URL: {}", e)),
    };

    let origin_param = query.get("origin").map(|s| s.as_str());
    let mut headers = generate_headers_for_url(&target_url_parsed, origin_param);

    if let Some(header_json) = query.get("headers") {
        if let Ok(parsed) = serde_json::from_str::<HashMap<String, String>>(header_json) {
            for (k, v) in parsed {
                if let (Ok(name), Ok(value)) = (
                    reqwest::header::HeaderName::from_bytes(k.as_bytes()),
                    reqwest::header::HeaderValue::from_str(&v),
                ) {
                    headers.insert(name, value);
                }
            }
        }
    }

    for header_name in &["range", "if-range", "if-none-match", "if-modified-since"] {
        if let Some(val) = req.headers().get(*header_name) {
            if let Ok(val_str) = val.to_str() {
                if let (Ok(name), Ok(value)) = (
                    reqwest::header::HeaderName::from_bytes(header_name.as_bytes()),
                    reqwest::header::HeaderValue::from_str(val_str),
                ) {
                    headers.insert(name, value);
                }
            }
        }
    }

    let is_range_req =
        req.headers().contains_key("range") || req.headers().contains_key("if-range");
    let cache_key = target_url.clone();

    if !is_range_req {
        if let Some(cached) = ASSET_CACHE.get(&cache_key) {
            let mut builder = HttpResponse::Ok();
            set_cors_headers(&mut builder, &acao);
            builder.insert_header(("X-Cache", "HIT"));
            for (k, v) in &cached.headers {
                builder.insert_header((k.clone(), v.clone()));
            }
            return builder.body(cached.body);
        }
    }

    let resp = match CLIENT.get(&target_url).headers(headers).send().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Upstream fetch failed for {}: {:?}", target_url, e);
            return HttpResponse::BadGateway().body("Failed to fetch upstream URL");
        }
    };

    let status = resp.status();
    let resp_headers = resp.headers().clone();
    let content_type = resp_headers
        .get("Content-Type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();

    let ct_is_m3u8 = content_type.contains("mpegurl")
        || content_type.contains("application/vnd.apple.mpegurl")
        || content_type.contains("application/x-mpegurl");
    let url_looks_m3u8 = target_url.to_ascii_lowercase().ends_with(".m3u8");

    if ct_is_m3u8 || url_looks_m3u8 {
        return handle_pipe(resp, &target_url, &target_url_parsed, &acao, origin_param).await;
    }

    let len_opt = resp.content_length();

    if !is_range_req && status == reqwest::StatusCode::OK {
        if let Some(len) = len_opt {
            if len <= 5_000_000 {
                let mut forwarded_headers = std::collections::HashMap::new();
                for (name, value) in resp_headers.iter() {
                    let header_name = name.as_str().to_ascii_lowercase();
                    if FORWARDED_HEADERS.contains(&header_name.as_str()) {
                        if let Ok(val_str) = value.to_str() {
                            forwarded_headers.insert(header_name, val_str.to_string());
                        }
                    }
                }

                if let Ok(bytes) = resp.bytes().await {
                    let cached_asset = CachedAsset {
                        headers: forwarded_headers.clone(),
                        body: bytes.clone(),
                    };
                    ASSET_CACHE.insert(cache_key, cached_asset);

                    let mut builder = HttpResponse::Ok();
                    set_cors_headers(&mut builder, &acao);
                    builder.insert_header(("X-Cache", "MISS"));
                    for (k, v) in &forwarded_headers {
                        builder.insert_header((k.clone(), v.clone()));
                    }
                    return builder.body(bytes);
                } else {
                    return HttpResponse::BadGateway().body("Failed to read upstream body");
                }
            }
        }
    }

    stream_response(resp, status, &resp_headers, &acao)
}

async fn handle_pipe(
    resp: reqwest::Response,
    target_url: &str,
    target_url_parsed: &Url,
    acao: &Option<String>,
    origin_param: Option<&str>,
) -> HttpResponse {
    if let Some(cached) = M3U8_CACHE.get(&target_url.to_string()) {
        let mut builder = HttpResponse::Ok();
        set_cors_headers(&mut builder, acao);
        builder.insert_header(("Content-Type", "application/vnd.apple.mpegurl"));
        builder.insert_header(("Cache-Control", "no-cache, no-store, must-revalidate"));
        builder.insert_header(("X-Cache", "HIT"));
        return builder.body(cached);
    }

    let m3u8_text = match resp.text().await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to read pipe body ({}): {:?}", target_url, e);
            return HttpResponse::InternalServerError().body("Failed to read pipe");
        }
    };

    if !m3u8_text.trim_start().starts_with("#EXTM3U") {
        let mut builder = HttpResponse::Ok();
        set_cors_headers(&mut builder, acao);
        return builder.body(m3u8_text);
    }

    let processed = process_pipe_body(&m3u8_text, target_url_parsed, origin_param);
    M3U8_CACHE.insert(target_url.to_string(), processed.clone());

    let mut builder = HttpResponse::Ok();
    set_cors_headers(&mut builder, acao);
    builder.insert_header(("Content-Type", "application/vnd.apple.mpegurl"));
    builder.insert_header(("Cache-Control", "no-cache, no-store, must-revalidate"));
    builder.insert_header(("X-Cache", "MISS"));
    builder.body(processed)
}

fn stream_response(
    resp: reqwest::Response,
    status: reqwest::StatusCode,
    resp_headers: &reqwest::header::HeaderMap,
    acao: &Option<String>,
) -> HttpResponse {
    let mut builder = HttpResponse::build(
        actix_web::http::StatusCode::from_u16(status.as_u16())
            .unwrap_or(actix_web::http::StatusCode::OK),
    );

    set_cors_headers(&mut builder, acao);

    for (name, value) in resp_headers.iter() {
        let header_name = name.as_str().to_lowercase();
        if FORWARDED_HEADERS.contains(&header_name.as_str()) {
            if let Ok(val_str) = value.to_str() {
                builder.insert_header((header_name.as_str().to_string(), val_str.to_string()));
            }
        }
    }

    let content_length = resp.content_length();

    // Stream upstream socket directly to client without buffering
    let stream = resp.bytes_stream().map(|chunk| {
        chunk
            .map(Bytes::from)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    });

    if let Some(len) = content_length {
        builder.body(actix_web::body::SizedStream::new(len, stream))
    } else {
        builder.body(actix_web::body::BodyStream::new(stream))
    }
}
