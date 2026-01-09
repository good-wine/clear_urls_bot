use teloxide::{prelude::*, types::{MessageEntityKind, ParseMode}, utils::html};
use crate::{sanitizer::RuleEngine, db::Db, models::ChatConfig};
use url::Url;

pub async fn run_bot(bot: Bot, db: Db, rules: RuleEngine, config: crate::config::Config) {
    let handler = Update::filter_message()
        .endpoint(handle_message);

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![db, rules, config])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

async fn handle_message(
    bot: Bot,
    msg: Message,
    db: Db,
    rules: RuleEngine,
    config: crate::config::Config,
) -> ResponseResult<()> {
    let chat_id = msg.chat.id;
    let user_id = msg.from().map(|u| u.id.0 as i64).unwrap_or(0);

    // Handle /start command in private chat
    if let Some(text) = msg.text() {
        if text.starts_with('/') && msg.chat.is_private() {
            match text {
                "/start" => {
                    let keyboard = teloxide::types::InlineKeyboardMarkup::new(vec![vec![
                        teloxide::types::InlineKeyboardButton::url(
                            "🚀 Apri Dashboard",
                            config.dashboard_url.parse().unwrap(),
                        )
                    ]]);

                    bot.send_message(chat_id, "<b>Benvenuto nel gestore ClearURLs!</b>\n\nPuoi configurare il bot e gestire i tuoi link puliti direttamente dal dashboard web protetto.")
                        .parse_mode(ParseMode::Html)
                        .reply_markup(keyboard)
                        .await?;
                    return Ok(());
                }
                "/help" => {
                    let help_text = "<b>Guida ClearURLs Bot</b> 🛡️\n\n\
                        Questo bot rimuove automaticamente i parametri di tracciamento dai link che invii.\n\n\
                        <b>Comandi:</b>\n\
                        /start - Inizia e ricevi il link al dashboard\n\
                        /help - Mostra questo messaggio\n\
                        /stats - Visualizza le tue statistiche di pulizia\n\n\
                        Puoi usarmi in chat privata o aggiungermi ai gruppi!";
                    bot.send_message(chat_id, help_text).parse_mode(ParseMode::Html).await?;
                    return Ok(());
                }
                "/stats" => {
                    let user_config = db.get_user_config(user_id).await.unwrap_or_default();
                    let stats_text = format!(
                        "<b>Le tue statistiche</b> 📊\n\n\
                        Link puliti finora: <b>{}</b>\n\n\
                        Grazie per proteggere la tua privacy!",
                        user_config.cleaned_count
                    );
                    bot.send_message(chat_id, stats_text).parse_mode(ParseMode::Html).await?;
                    return Ok(());
                }
                _ => {}
            }
        }
    }
    // Persist/Update chat info
    if msg.chat.is_group() || msg.chat.is_supergroup() || msg.chat.is_channel() {
        let title = msg.chat.title().map(|s| s.to_string());
        let _ = db.save_chat_config(&ChatConfig {
            chat_id: chat_id.0,
            title,
            enabled: true,
            added_by: user_id,
        }).await;
    }

    let chat_config = db.get_chat_config(chat_id.0).await.ok();
    let user_config = db.get_user_config(user_id).await.ok();

    let chat_enabled = chat_config.map(|c| c.enabled).unwrap_or(true);
    let user_enabled = user_config.as_ref().map(|c| c.enabled).unwrap_or(true);

    if !chat_enabled || !user_enabled {
        return Ok(())
    }

    let (text, entities) = if let Some(t) = msg.text() {
        (t, msg.entities())
    } else if let Some(c) = msg.caption() {
        (c, msg.caption_entities())
    } else {
        return Ok(());
    };

    let entities = match entities {
        Some(e) => e,
        None => return Ok(()),
    };

    let ignored_domains: Vec<String> = user_config.as_ref()
        .map(|c| c.ignored_domains.split(',').map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()).collect())
        .unwrap_or_default();

    let mut cleaned_urls = Vec::new();
    let utf16: Vec<u16> = text.encode_utf16().collect();

    for entity in entities {
        let url_str = match &entity.kind {
            MessageEntityKind::Url => {
                let start = entity.offset;
                let end = start + entity.length;
                if end > utf16.len() { continue; }
                String::from_utf16_lossy(&utf16[start..end])
            },
            MessageEntityKind::TextLink { url } => {
                url.to_string()
            },
            _ => continue,
        };

        // Domain check
        if let Ok(parsed) = Url::parse(&url_str) {
            if let Some(host) = parsed.host_str() {
                if ignored_domains.iter().any(|d| host.contains(d)) {
                    continue;
                }
            }
        }

        if let Some(cleaned) = rules.sanitize(&url_str) {
             if cleaned != url_str {
                 cleaned_urls.push((url_str, cleaned));
             }
        }
    }

    if cleaned_urls.is_empty() {
        return Ok(())
    }

    // Increment stats
    let _ = db.increment_cleaned_count(user_id, cleaned_urls.len() as i64).await;

    let mode = user_config.map(|c| c.mode).unwrap_or("reply".to_string());

    if mode == "delete" {
         if bot.delete_message(chat_id, msg.id).await.is_ok() {
             let user_name = msg.from().map(|u| u.first_name.clone()).unwrap_or("User".into());
             let mut response = format!("<b>Link(s) cleaned for {}:</b>\n", html::escape(&user_name));
             for (_, cleaned) in cleaned_urls {
                 response.push_str(&format!("• <a href=\"{}\">{}</a>\n", html::escape(&cleaned), html::escape(&cleaned)));
             }
             bot.send_message(chat_id, response).parse_mode(ParseMode::Html).await?;
             return Ok(());
         }
    }

    let mut response = String::from("<b>Cleaned Link(s):</b>\n");
    for (_, cleaned) in cleaned_urls {
         response.push_str(&format!("• {}\n", html::escape(&cleaned)));
    }
    
    bot.send_message(chat_id, response)
        .reply_to_message_id(msg.id)
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(())
}
