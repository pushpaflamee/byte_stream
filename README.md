## Route

```bash
# you can also add origin parameter to the url, its optional use in case it gives 403.
GET /?u={encrypted_url}&origin={origin}
```

## Scripts

```bash
# Encrypt a URL (Node.js) for testing purpose
node scripts/enc.js "https://example.com/video.m3u8"
```


## credits
shoutout to the following repos which provided inspiration and foundational logic:
- [zuhaz/rust-proxy](https://github.com/zuhaz/rust-proxy)
- [Rob--W/cors-anywhere](https://github.com/Rob--W/cors-anywhere)
- [Eltik/M3U8-Proxy](https://github.com/Eltik/M3U8-Proxy)
- [Chance/CloudflareWorkerProxy](https://github.com/Gratenes/m3u8CloudflareWorkerProxy)