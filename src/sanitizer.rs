use std::collections::HashMap;
use regex::Regex;
use serde::Deserialize;
use url::Url;
use anyhow::{Result, Context};
use tracing::info;
use std::sync::{Arc, RwLock};

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
    pub async fn new(source_url: &str) -> Result<Self> {
        let engine = Self {
            providers: Arc::new(RwLock::new(Vec::new())),
            source_url: source_url.to_string(),
        };
        engine.refresh().await?;
        Ok(engine)
    }

    pub async fn refresh(&self) -> Result<()> {
        info!("Fetching rules from {}", self.source_url);
        let client = reqwest::Client::new();
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
            let mut w = self.providers.write().unwrap();
            *w = compiled_providers;
        }
        
        info!("Loaded {} providers", count);
        Ok(())
    }

    pub fn sanitize(&self, text: &str, custom_rules: &[crate::models::CustomRule], ignored_domains: &[String]) -> Option<(String, String)> {
        if let Ok(mut url) = Url::parse(text) {
             if let Some(host) = url.host_str() {
                 if ignored_domains.iter().any(|d| host.contains(d)) {
                     return None;
                 }
             }

             let mut provider_name = String::from("Custom/Other");
             
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
                 let providers = self.providers.read().unwrap();
                 for p in providers.iter() {
                     if p.url_pattern.is_match(text) {
                         provider_name = p.name.clone();
                         break;
                     }
                 }
             }

             // 3. Apply Extended Algorithm
             if self.clean_url_in_place(&mut url) || custom_changed {
                 return Some((url.to_string(), provider_name));
             }
        }
        None
    }

    pub fn clean_url_in_place(&self, url: &mut Url) -> bool {
        let mut changed = false;
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 5;

        while iterations < MAX_ITERATIONS {
            let url_str = url.to_string();
            let mut current_iteration_changed = false;

            let providers = self.providers.read().unwrap();
            
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

            if !current_iteration_changed {
                break;
            }
            iterations += 1;
        }
        changed
    }
}