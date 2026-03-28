use crate::config::{ALLOWED_ORIGINS, ENABLE_CORS};
use actix_web::{http::header, HttpRequest, HttpResponse, Responder};

pub fn get_valid_origin(req: &HttpRequest) -> Option<String> {
    if !*ENABLE_CORS {
        return None;
    }

    if let Some(origin) = req.headers().get(header::ORIGIN) {
        if let Ok(origin_str) = origin.to_str() {
            if ALLOWED_ORIGINS.iter().any(|o| o == origin_str) {
                return Some(origin_str.to_string());
            }
        }
    }

    if let Some(referer) = req.headers().get(header::REFERER) {
        if let Ok(referer_str) = referer.to_str() {
            if let Some(allowed) = ALLOWED_ORIGINS
                .iter()
                .find(|origin| referer_str.starts_with(origin.as_str()))
            {
                return Some(allowed.clone());
            }
        }
    }

    None
}

// also got from zuhaz

#[inline]
pub fn set_cors_headers(builder: &mut actix_web::HttpResponseBuilder, origin: &Option<String>) {
    let acao = origin.as_deref().unwrap_or("*");
    builder.insert_header((header::ACCESS_CONTROL_ALLOW_ORIGIN, acao));
    builder.insert_header((
        header::ACCESS_CONTROL_ALLOW_METHODS,
        "GET, POST, OPTIONS, HEAD",
    ));
    builder.insert_header((
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        "Content-Type, Authorization, Range, X-Requested-With, Origin, Accept, Accept-Encoding, Accept-Language, Cache-Control, Pragma, Sec-Fetch-Dest, Sec-Fetch-Mode, Sec-Fetch-Site, Sec-Ch-Ua, Sec-Ch-Ua-Mobile, Sec-Ch-Ua-Platform, Connection",
    ));
    builder.insert_header((
        header::ACCESS_CONTROL_EXPOSE_HEADERS,
        "Content-Length, Content-Range, Accept-Ranges, Content-Type, Cache-Control, Expires, Vary, ETag, Last-Modified",
    ));
    builder.insert_header((header::CROSS_ORIGIN_RESOURCE_POLICY, "cross-origin"));
    builder.insert_header(("Vary", "Origin"));
}

pub async fn handle_options(req: HttpRequest) -> impl Responder {
    let origin = match get_valid_origin(&req) {
        Some(o) => o,
        None => {
            if *ENABLE_CORS {
                return HttpResponse::Forbidden().finish();
            }
            "*".to_string()
        }
    };

    let mut resp = HttpResponse::Ok();
    set_cors_headers(&mut resp, &Some(origin));
    resp.insert_header((header::ACCESS_CONTROL_MAX_AGE, "86400"));
    resp.finish()
}
