use actix_web::web::Bytes;
use mini_moka::sync::Cache;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Clone)]
pub struct CachedAsset {
    pub headers: HashMap<String, String>,
    pub body: Bytes,
}

pub static M3U8_CACHE: Lazy<Cache<String, String>> = Lazy::new(|| {
    Cache::builder()
        .max_capacity(128)
        .time_to_live(Duration::from_secs(4))
        .time_to_idle(Duration::from_secs(15))
        .build()
});

pub static ASSET_CACHE: Lazy<Cache<String, CachedAsset>> = Lazy::new(|| {
    Cache::builder()
        .weigher(|_k, v: &CachedAsset| v.body.len().try_into().unwrap_or(u32::MAX))
        .max_capacity(250 * 1024 * 1024)
        .time_to_live(Duration::from_secs(60))
        .time_to_idle(Duration::from_secs(30))
        .build()
});
