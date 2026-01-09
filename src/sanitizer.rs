use std::collections::HashMap;
use regex::Regex;
use serde::Deserialize;
use url::Url;
use anyhow::{Result, Context};
use tracing::{info, debug};

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
    providers: std::sync::Arc<Vec<CompiledProvider>>,
}

impl RuleEngine {
    pub async fn new(source_url: &str) -> Result<Self> {
        info!("Fetching rules from {}", source_url);
        let client = reqwest::Client::new();
        let resp = client.get(source_url).send().await?.text().await?;
        
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

        info!("Loaded {} providers", compiled_providers.len());

        Ok(Self {
            providers: std::sync::Arc::new(compiled_providers),
        })
    }

    pub fn sanitize(&self, text: &str) -> Option<String> {
        if let Ok(mut url) = Url::parse(text) {
             if self.clean_url_in_place(&mut url) {
                 return Some(url.to_string());
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

            for provider in self.providers.iter() {
                if provider.url_pattern.is_match(&url_str) {
                    debug!("Matched provider: {}", provider.name);

                    for exception in &provider.exceptions {
                        if exception.is_match(&url_str) {
                            return changed;
                        }
                    }

                    // Handle redirections
                    for redirection_regex in &provider.redirections {
                         if let Some(caps) = redirection_regex.captures(&url_str) {
                             if let Some(m) = caps.get(1) {
                                 if let Ok(new_url) = Url::parse(m.as_str()) {
                                     *url = new_url;
                                     current_iteration_changed = true;
                                     changed = true;
                                     break;
                                 }
                             }
                         }
                    }
                    
                    if current_iteration_changed { break; }

                    let query_pairs: Vec<(String, String)> = url.query_pairs().into_owned().collect();
                    let mut new_query = url::form_urlencoded::Serializer::new(String::new());
                    let mut has_params = false;
                    let mut params_removed = false;

                    for (key, value) in query_pairs {
                        let mut keep = true;
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
                            new_query.append_pair(&key, &value);
                            has_params = true;
                        } else {
                            params_removed = true;
                        }
                    }

                    if params_removed {
                        changed = true;
                        current_iteration_changed = true;
                        if has_params {
                            url.set_query(Some(&new_query.finish()));
                        } else {
                            url.set_query(None);
                        }
                    }
                    
                     let mut intermediate_url_str = url.to_string();
                     for raw in &provider.raw_rules {
                         let new_str = raw.replace_all(&intermediate_url_str, "");
                         if new_str != intermediate_url_str {
                             intermediate_url_str = new_str.to_string();
                             changed = true;
                             current_iteration_changed = true;
                         }
                     }
                     
                     if current_iteration_changed {
                         if let Ok(new_url) = Url::parse(&intermediate_url_str) {
                             *url = new_url;
                         }
                         break;
                     }

                    return changed;
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
