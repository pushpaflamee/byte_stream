use once_cell::sync::Lazy;

pub static XOR_KEY: Lazy<Vec<u8>> = Lazy::new(|| {
    std::env::var("XOR_KEY")
        .unwrap_or_else(|_| "s3cr3t_k3y_pr0xy".to_string())
        .into_bytes()
});

pub static PORT: Lazy<u16> = Lazy::new(|| {
    std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8080)
});

pub static ALLOWED_ORIGINS: Lazy<Vec<String>> = Lazy::new(|| {
    std::env::var("ALLOWED_ORIGIN")
        .unwrap_or_else(|_| "http://localhost:8080".to_string())
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
});

pub static ENABLE_CORS: Lazy<bool> = Lazy::new(|| {
    std::env::var("ENABLE_CORS")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false)
});
