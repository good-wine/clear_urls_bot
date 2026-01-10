use wasm_bindgen::prelude::*;
use url::Url;

#[wasm_bindgen]
pub fn clean_url_simple(input_url: &str) -> String {
    let mut url = match Url::parse(input_url) {
        Ok(u) => u,
        Err(_) => return input_url.to_string(),
    };

    let query_pairs: Vec<(String, String)> = url.query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();

    let mut new_query = Vec::new();
    let tracking_params = [
        "utm_source", "utm_medium", "utm_campaign", "utm_term", "utm_content",
        "fbclid", "gclid", "msclkid", "mc_eid", "_hsenc", "_hsmi",
        "gs_lcrp", "oq", "sourceid", "client", "bih", "biw", "ved", "ei",
        "iflsig", "adgrpid", "nw", "matchtype"
    ];

    for (k, v) in query_pairs {
        if !tracking_params.contains(&k.as_str()) {
            new_query.push((k, v));
        }
    }

    if new_query.is_empty() {
        url.set_query(None);
    } else {
        let mut query_str = String::new();
        for (i, (k, v)) in new_query.iter().enumerate() {
            if i > 0 { query_str.push('&'); }
            query_str.push_str(k);
            query_str.push('=');
            query_str.push_str(v);
        }
        url.set_query(Some(&query_str));
    }

    url.to_string()
}
