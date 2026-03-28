use crate::config::XOR_KEY;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};

#[inline(always)]
fn xor_transform(data: &mut [u8]) {
    let key = XOR_KEY.as_slice();
    let key_len = key.len();
    for (i, byte) in data.iter_mut().enumerate() {
        *byte ^= key[i % key_len];
    }
}

pub fn encrypt_url(url: &str) -> String {
    let mut data = url.as_bytes().to_vec();
    xor_transform(&mut data);
    URL_SAFE_NO_PAD.encode(&data)
}

pub fn decrypt_url(encrypted: &str) -> Option<String> {
    let mut data = URL_SAFE_NO_PAD.decode(encrypted).ok()?;
    xor_transform(&mut data);
    String::from_utf8(data).ok()
}
