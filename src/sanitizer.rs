use std::collections::HashMap;
use regex::Regex;
use serde::Deserialize;
use url::Url;
use anyhow::{Result, Context};
use tracing::info;
use std::sync::{Arc, RwLock, LazyLock};

static SENSITIVE_PATTERNS: LazyLock<HashMap<&'static str, Regex>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    // Use \b (word boundary) instead of look-arounds
    m.insert("aws_access_key", Regex::new(r"(?i)\b[A-Z0-9]{20}\b").unwrap());
    m.insert("aws_secret_key", Regex::new(r"(?i)\b[A-Za-z0-9/+=]{40}\b").unwrap());
    m.insert("password", Regex::new(r"(?i)password\s*[:=]\s*[^\s]+").unwrap());
    m.insert("ipv4", Regex::new(r"(?:\d{1,3}\.){3}\d{1,3}").unwrap());
    m.insert("email", Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap());
    m
});

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct RawProvider {
    #[serde(default)]
    urlPattern: String,
    #[serde(default)]
    rules: Vec<String>,
    #[serde(default)]
    exceptions: Vec<String>,
    #[serde(default)]
    rawRules: Vec<String>,
    #[serde(default)]
    redirections: Vec<String>,
    #[serde(default)]
    referralMarketing: Vec<String>,
    #[serde(default)]
    forceRedirection: bool,
}

#[derive(Debug, Deserialize)]
struct ClearUrlsData {
    providers: HashMap<String, RawProvider>,
}

#[derive(Clone)]
struct CompiledProvider {
    name: String,
    url_pattern: Regex,
    rules: Vec<Regex>,
    exceptions: Vec<Regex>,
    raw_rules: Vec<Regex>,
    redirections: Vec<Regex>,
    referral_marketing: Vec<Regex>,
    _force_redirection: bool,
}

#[derive(Clone)]
pub struct RuleEngine {
    providers: Arc<RwLock<Vec<CompiledProvider>>>,
    source_url: String,
}

impl RuleEngine {
    pub fn new_lazy(source_url: &str) -> Self {
        Self {
            providers: Arc::new(RwLock::new(Vec::new())),
            source_url: source_url.to_string(),
        }
    }

    pub async fn new(source_url: &str) -> Result<Self> {
        let engine = Self::new_lazy(source_url);
        engine.refresh().await?;
        Ok(engine)
    }

    pub async fn refresh(&self) -> Result<()> {
        info!("Fetching rules from {}", self.source_url);
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        let resp = client.get(&self.source_url).send().await?.text().await?;
        
        let data: ClearUrlsData = serde_json::from_str(&resp).context("Failed to parse ClearURLs JSON")?;
        
        let mut compiled_providers = Vec::new();

        for (name, provider) in data.providers {
            if provider.urlPattern.is_empty() {
                continue;
            }

            let url_pattern = match Regex::new(&provider.urlPattern) {
                Ok(r) => r,
                Err(_) => continue,
            };

            let compile_list = |list: &[String]| -> Vec<Regex> {
                list.iter().filter_map(|s| Regex::new(s).ok()).collect()
            };

            compiled_providers.push(CompiledProvider {
                name,
                url_pattern,
                rules: compile_list(&provider.rules),
                exceptions: compile_list(&provider.exceptions),
                raw_rules: compile_list(&provider.rawRules),
                redirections: compile_list(&provider.redirections),
                referral_marketing: compile_list(&provider.referralMarketing),
                _force_redirection: provider.forceRedirection,
            });
        }

        let count = compiled_providers.len();
        {
            if let Ok(mut w) = self.providers.write() {
                *w = compiled_providers;
            } else {
                tracing::error!("Failed to acquire write lock for providers");
                return Err(anyhow::anyhow!("Lock error"));
            }
        }
        
        info!("Loaded {} providers", count);
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn expand_url(&self, input_url: &str) -> String {
        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::limited(5))
            .build() {
                Ok(c) => c,
                Err(_) => return input_url.to_string(),
            };

        // We only want to expand common shorteners or if it looks like a redirector
        let url_lower = input_url.to_lowercase();
        let shorteners = ["bit.ly", "tinyurl.com", "t.co", "goo.gl", "rebrand.ly", "buff.ly", "is.gd", "ow.ly", "t.me", "shorturl.at"];
        
        let is_shortener = shorteners.iter().any(|s| url_lower.contains(s));
        
        if is_shortener {
            tracing::debug!(url = %input_url, "Attempting to expand shortened URL");
            if let Ok(resp) = client.head(input_url).send().await {
                let final_url = resp.url().to_string();
                if final_url != input_url {
                    tracing::info!(original = %input_url, expanded = %final_url, "URL expanded successfully");
                    return final_url;
                }
            }
        }
        input_url.to_string()
    }

    pub fn redact_sensitive(&self, text: &str) -> String {
        let mut redacted = text.to_string();
        for (name, re) in SENSITIVE_PATTERNS.iter() {
            redacted = re.replace_all(&redacted, format!("[REDACTED {}]", name.to_uppercase())).to_string();
        }
        redacted
    }

    fn clean_github_url(&self, url: &mut Url) -> bool {
        if let Some(host) = url.host_str() {
            if host == "github.com" {
                let path_segments: Vec<String> = url.path_segments()
                    .map(|s| s.map(String::from).collect())
                    .unwrap_or_default();
                
                // If it's a deep link (e.g. /owner/repo/blob/main/file.ext), truncate to /owner/repo
                if path_segments.len() > 2 {
                    let owner = &path_segments[0];
                    let repo = &path_segments[1];
                    let new_path = format!("/{}/{}", owner, repo);
                    if url.path() != new_path {
                        url.set_path(&new_path);
                        url.set_query(None);
                        url.set_fragment(None);
                        return true;
                    }
                }
            }
        }
        false
    }

    #[tracing::instrument(skip(self, custom_rules, ignored_domains))]
    pub fn sanitize(&self, text: &str, custom_rules: &[crate::models::CustomRule], ignored_domains: &[String]) -> Option<(String, String)> {
        tracing::debug!(url = %self.redact_sensitive(text), "Starting sanitization");
        
        let mut url_to_parse = text.to_string();
        if !url_to_parse.contains("://") && !url_to_parse.starts_with("mailto:") {
            url_to_parse = format!("http://{}", url_to_parse);
        }

        if let Ok(mut url) = Url::parse(&url_to_parse) {
             if let Some(host) = url.host_str() {
                 if ignored_domains.iter().any(|d| host.contains(d)) {
                     tracing::debug!(host = %host, "URL host is in ignored domains");
                     return None;
                 }
             }

             let mut provider_name = String::from("Custom/Other");
             let github_changed = self.clean_github_url(&mut url);
             if github_changed {
                 provider_name = "GitHub (Repo Root)".to_string();
             }
             
             // 1. Apply Custom User Rules FIRST
             let mut custom_changed = false;
             if let Some(_query) = url.query() {
                 let query_pairs: Vec<(String, String)> = url.query_pairs().into_owned().collect();
                 let mut new_query = url::form_urlencoded::Serializer::new(String::new());
                 let mut any_kept = false;
                 
                 for (key, value) in query_pairs {
                     let mut keep = true;
                     for crule in custom_rules {
                         if key.contains(&crule.pattern) {
                             keep = false;
                             custom_changed = true;
                             tracing::debug!(param = %key, rule = %crule.pattern, "Custom rule matched");
                             break;
                         }
                     }
                     if keep {
                         new_query.append_pair(&key, &value);
                         any_kept = true;
                     }
                 }

                 if custom_changed {
                     if any_kept {
                         url.set_query(Some(&new_query.finish()));
                     } else {
                         url.set_query(None);
                     }
                 }
             }

             // 2. Identify Provider
             {
                 if let Ok(providers) = self.providers.read() {
                     for p in providers.iter() {
                         if p.url_pattern.is_match(text) {
                             provider_name = p.name.clone();
                             tracing::debug!(provider = %provider_name, "Provider identified");
                             break;
                         }
                     }
                 }
             }

             // 3. Apply Extended Algorithm
             let mut changed = self.clean_url_in_place(&mut url);

             // 4. Aggressive Fallback for common trackers not in the ruleset
             // (e.g. Google Search gs_lcrp, oq, client, etc.)
             if let Some(_query) = url.query() {
                 let query_pairs: Vec<(String, String)> = url.query_pairs().into_owned().collect();
                 let mut new_query = url::form_urlencoded::Serializer::new(String::new());
                 let mut aggressive_changed = false;
                 let mut any_kept = false;

                 let aggressive_trackers = [
                     "gs_lcrp", "oq", "sourceid", "client", "bih", "biw", "ved", "ei", "iflsig", "adgrpid", "nw", "matchtype"
                 ];

                 for (key, value) in query_pairs {
                     if aggressive_trackers.contains(&key.as_str()) {
                         aggressive_changed = true;
                         tracing::debug!(param = %key, "Aggressive tracker stripped");
                         continue;
                     }
                     new_query.append_pair(&key, &value);
                     any_kept = true;
                 }

                 if aggressive_changed {
                     changed = true;
                     if any_kept {
                         url.set_query(Some(&new_query.finish()));
                     } else {
                         url.set_query(None);
                     }
                 }
             }

             if changed || custom_changed || github_changed {
                 let cleaned = url.to_string();
                 tracing::info!(
                     original = %self.redact_sensitive(text), 
                     cleaned = %cleaned, 
                     provider = %provider_name, 
                     "URL successfully cleaned"
                 );
                 return Some((cleaned, provider_name));
             }
        }
        None
    }

    #[tracing::instrument(skip(self, url))]
    pub fn clean_url_in_place(&self, url: &mut Url) -> bool {
        let mut changed = false;
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 5;

        while iterations < MAX_ITERATIONS {
            let url_str = url.to_string();
            let mut current_iteration_changed = false;

            if let Ok(providers) = self.providers.read() {
                // 1. Match specific providers AND the global/generic one if it exists
                for provider in providers.iter() {
                    // "generic" provider usually matches everything or has a catch-all pattern
                    if provider.url_pattern.is_match(&url_str) || provider.name == "generic" {
                        
                        let mut provider_changed = false;

                        // Exceptions check
                        let mut is_exception = false;
                        for exception in &provider.exceptions {
                            if exception.is_match(&url_str) {
                                is_exception = true;
                                break;
                            }
                        }
                        if is_exception { continue; }

                        // Handle redirections
                        for redirection_regex in &provider.redirections {
                            if let Some(caps) = redirection_regex.captures(&url_str) {
                                if let Some(m) = caps.get(1) {
                                    if let Ok(new_url) = Url::parse(m.as_str()) {
                                        *url = new_url;
                                        current_iteration_changed = true;
                                        provider_changed = true;
                                        changed = true;
                                        break;
                                    }
                                }
                            }
                        }
                        
                        if provider_changed { continue; }

                        // Handle Query Parameters
                        if let Some(_query) = url.query() {
                            let query_pairs: Vec<(String, String)> = url.query_pairs().into_owned().collect();
                            let mut new_query = url::form_urlencoded::Serializer::new(String::new());
                            let mut params_removed = false;
                            let mut any_kept = false;

                            for (key, mut value) in query_pairs {
                                let mut keep = true;
                                
                                // Apply rules
                                for rule in &provider.rules {
                                    if rule.is_match(&key) {
                                        keep = false;
                                        break;
                                    }
                                }
                                if keep {
                                    for rule in &provider.referral_marketing {
                                        if rule.is_match(&key) {
                                            keep = false;
                                            break;
                                        }
                                    }
                                }

                                if keep {
                                    // Recursive cleaning: check if value is a URL
                                    if value.starts_with("http") {
                                        if let Ok(mut inner_url) = Url::parse(&value) {
                                            if self.clean_url_in_place(&mut inner_url) {
                                                value = inner_url.to_string();
                                                changed = true;
                                            }
                                        }
                                    }
                                    new_query.append_pair(&key, &value);
                                    any_kept = true;
                                } else {
                                    params_removed = true;
                                }
                            }

                            if params_removed {
                                changed = true;
                                current_iteration_changed = true;
                                if any_kept {
                                    url.set_query(Some(&new_query.finish()));
                                } else {
                                    url.set_query(None);
                                }
                            }
                        }

                        // Handle Fragment (hash) - some tracking is after #
                        if let Some(fragment) = url.fragment() {
                            if fragment.contains('=') {
                                 // Try to parse fragment as query string
                                 let frag_url_str = format!("http://localhost?{}", fragment);
                                 if let Ok(mut frag_url) = Url::parse(&frag_url_str) {
                                     if self.clean_url_in_place(&mut frag_url) {
                                         if let Some(new_frag) = frag_url.query() {
                                             url.set_fragment(Some(new_frag));
                                         } else {
                                             url.set_fragment(None);
                                         }
                                         changed = true;
                                         current_iteration_changed = true;
                                     }
                                 }
                            }
                        }
                        
                        // Raw rules
                        let mut intermediate_url_str = url.to_string();
                        let mut raw_changed = false;
                        for raw in &provider.raw_rules {
                            let new_str = raw.replace_all(&intermediate_url_str, "");
                            if new_str != intermediate_url_str {
                                intermediate_url_str = new_str.to_string();
                                raw_changed = true;
                            }
                        }
                        
                        if raw_changed {
                            if let Ok(new_url) = Url::parse(&intermediate_url_str) {
                                *url = new_url;
                                changed = true;
                                current_iteration_changed = true;
                            }
                        }
                    }
                }
            }

            if !current_iteration_changed {
                break;
            }
            iterations += 1;
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simple_cleaning() {
        let engine = RuleEngine {
            providers: Arc::new(RwLock::new(Vec::new())),
            source_url: String::new(),
        };
        
        // Mock a generic provider
        {
            let mut w = engine.providers.write().unwrap();
            w.push(CompiledProvider {
                name: "generic".to_string(),
                url_pattern: Regex::new(".*").unwrap(),
                rules: vec![Regex::new("utm_.*").unwrap()],
                exceptions: vec![],
                raw_rules: vec![],
                redirections: vec![],
                referral_marketing: vec![],
                _force_redirection: false,
            });
        }

        let input = "https://example.com/?utm_source=test&foo=bar";
        let (cleaned, _) = engine.sanitize(input, &[], &[]).unwrap();
        assert_eq!(cleaned, "https://example.com/?foo=bar");
    }
}