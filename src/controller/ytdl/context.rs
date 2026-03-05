use regex::Regex;
use waproto::whatsapp as wa;
use whatsapp_rust::bot::MessageContext;
use whatsapp_rust::proto_helpers::MessageExt;

fn looks_like_url(input: &str) -> bool {
    input.starts_with("http://") || input.starts_with("https://")
}

fn clean_url_candidate(input: &str) -> String {
    input
        .trim()
        .trim_matches(|c: char| {
            matches!(
                c,
                '"' | '\'' | '<' | '>' | ')' | '(' | '[' | ']' | '{' | '}' | ',' | '.'
            )
        })
        .to_string()
}

fn is_youtube_url(input: &str) -> bool {
    if !looks_like_url(input) {
        return false;
    }

    let lower = input.to_ascii_lowercase();
    lower.contains("://youtube.com/")
        || lower.contains("://www.youtube.com/")
        || lower.contains("://m.youtube.com/")
        || lower.contains("://music.youtube.com/")
        || lower.contains("://youtu.be/")
}

fn filter_youtube_urls(candidates: Vec<String>) -> Vec<String> {
    let mut urls = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for candidate in candidates {
        let cleaned = clean_url_candidate(&candidate);
        if !is_youtube_url(&cleaned) {
            continue;
        }
        if seen.insert(cleaned.clone()) {
            urls.push(cleaned);
        }
    }

    urls
}

pub fn extract_context_info(base: &wa::Message) -> Option<&wa::ContextInfo> {
    if let Some(ext) = &base.extended_text_message {
        return ext.context_info.as_deref();
    }
    if let Some(img) = &base.image_message {
        return img.context_info.as_deref();
    }
    if let Some(vid) = &base.video_message {
        return vid.context_info.as_deref();
    }
    if let Some(doc) = &base.document_message {
        return doc.context_info.as_deref();
    }
    None
}

pub fn urls_from_text(text: &str) -> Vec<String> {
    let re = Regex::new(r#"https?://[^\s<>"']+"#).expect("valid regex");
    re.find_iter(text).map(|m| m.as_str().to_string()).collect()
}

pub fn urls_from_quoted_message(ctx: &MessageContext) -> Vec<String> {
    let base = ctx.message.get_base_message();
    let Some(quoted) = extract_context_info(base).and_then(|i| i.quoted_message.as_ref()) else {
        return Vec::new();
    };

    let mut urls = Vec::new();
    if let Some(text) = quoted.conversation.as_deref() {
        urls.extend(urls_from_text(text));
    }
    if let Some(ext) = quoted.extended_text_message.as_ref()
        && let Some(text) = ext.text.as_deref()
    {
        urls.extend(urls_from_text(text));
    }
    if let Some(img) = quoted.image_message.as_ref()
        && let Some(caption) = img.caption.as_deref()
    {
        urls.extend(urls_from_text(caption));
    }
    if let Some(vid) = quoted.video_message.as_ref()
        && let Some(caption) = vid.caption.as_deref()
    {
        urls.extend(urls_from_text(caption));
    }
    if let Some(doc) = quoted.document_message.as_ref()
        && let Some(caption) = doc.caption.as_deref()
    {
        urls.extend(urls_from_text(caption));
    }
    urls
}

pub fn resolve_youtube_urls(ctx: &MessageContext, raw_args: &str) -> Vec<String> {
    let from_args = filter_youtube_urls(urls_from_text(raw_args));
    if !from_args.is_empty() {
        return from_args;
    }
    filter_youtube_urls(urls_from_quoted_message(ctx))
}
