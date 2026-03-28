use crate::crypto::encrypt_url;
use url::Url;

#[inline]
fn get_url(line: &str, base: &Url) -> Url {
    if let Ok(absolute) = Url::parse(line) {
        return absolute;
    }
    base.join(line).unwrap_or_else(|_| base.clone())
}

#[inline]
fn build_proxy_url(resolved: &Url, origin_param: Option<&str>) -> String {
    let encrypted = encrypt_url(resolved.as_str());
    let mut result = String::with_capacity(encrypted.len() + 50);
    result.push_str("/?u=");
    result.push_str(&encrypted);

    if let Some(o) = origin_param {
        result.push_str("&origin=");
        result.push_str(&urlencoding::encode(o));
    }

    result
}

pub fn process_pipe_line(line: &str, base_url: &Url, origin_param: Option<&str>) -> String {
    if line.is_empty() {
        return String::new();
    }

    if line.starts_with('#') {
        if line.len() > 11 && line.starts_with("#EXT-X-KEY") {
            if let Some(uri_start) = line.find("URI=\"") {
                let key_uri_start = uri_start + 5;
                if let Some(quote_pos) = line[key_uri_start..].find('"') {
                    let key_uri_end = key_uri_start + quote_pos;
                    let key_uri = &line[key_uri_start..key_uri_end];
                    let resolved = get_url(key_uri, base_url);
                    let proxy_url = build_proxy_url(&resolved, origin_param);

                    let mut result = String::with_capacity(line.len() + proxy_url.len());
                    result.push_str(&line[..key_uri_start]);
                    result.push_str(&proxy_url);
                    result.push_str(&line[key_uri_end..]);
                    return result;
                }
            }
            return line.to_string();
        }

        if line.len() > 16 && line.starts_with("#EXT-X-MAP:URI=\"") {
            let inner_url = &line[16..line.len() - 1];
            let resolved = get_url(inner_url, base_url);
            let proxy_url = build_proxy_url(&resolved, origin_param);

            let mut result = String::from("#EXT-X-MAP:URI=\"");
            result.push_str(&proxy_url);
            result.push('"');
            return result;
        }

        if line.len() > 20 && (line.contains("URI=") || line.contains("URL=")) {
            if let Some(colon_pos) = line.find(':') {
                let prefix = &line[..colon_pos + 1];
                let attrs = &line[colon_pos + 1..];

                let mut result = String::with_capacity(line.len() + 100);
                result.push_str(prefix);

                let mut in_quotes = false;
                let mut current_attr = String::new();
                let mut parsed_attrs = Vec::new();

                for c in attrs.chars() {
                    if c == '"' {
                        in_quotes = !in_quotes;
                    }
                    if c == ',' && !in_quotes {
                        parsed_attrs.push(current_attr.clone());
                        current_attr.clear();
                    } else {
                        current_attr.push(c);
                    }
                }
                parsed_attrs.push(current_attr);

                let mut first_attr = true;
                for attr in parsed_attrs {
                    if !first_attr {
                        result.push(',');
                    }
                    first_attr = false;

                    if let Some(eq_pos) = attr.find('=') {
                        let key = attr[..eq_pos].trim();
                        let value = attr[eq_pos + 1..].trim().trim_matches('"');

                        if key == "URI" || key == "URL" {
                            let resolved = get_url(value, base_url);
                            let proxy_url = build_proxy_url(&resolved, origin_param);
                            result.push_str(key);
                            result.push_str("=\"");
                            result.push_str(&proxy_url);
                            result.push('"');
                        } else {
                            result.push_str(&attr);
                        }
                    } else {
                        result.push_str(&attr);
                    }
                }
                return result;
            }
        }

        return line.to_string();
    }

    let resolved = get_url(line, base_url);
    build_proxy_url(&resolved, origin_param)
}

pub fn process_pipe_body(body: &str, base_url: &Url, origin_param: Option<&str>) -> String {
    let lines: Vec<&str> = body.lines().collect();
    let mut result = Vec::with_capacity(lines.len());
    for line in lines {
        result.push(process_pipe_line(line, base_url, origin_param));
    }
    result.join("\n")
}
