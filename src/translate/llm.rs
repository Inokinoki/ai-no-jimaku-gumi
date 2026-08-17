use anyhow::Result;
use genai::chat::{ChatMessage, ChatRequest};
use genai::Client;

const NUMBERED_LIST_INSTRUCTIONS: &str = " The texts are given as a numbered list, one subtitle per line, \
in the format `N: text`. Reply with exactly the same amount of lines, one translated text per line, \
keeping the original `N: ` prefix for each line. Do not merge, split, reorder or omit any lines.";

fn build_numbered_list(texts: &[String]) -> String {
    texts
        .iter()
        .enumerate()
        .map(|(i, text)| format!("{}: {}", i + 1, text))
        .collect::<Vec<_>>()
        .join("\n")
}

// Lines are matched by their `N: ` prefix so that a missing or malformed
// line only affects itself: unparsed lines fall back to the original text.
fn parse_numbered_list(content: &str, texts: &[String]) -> Vec<String> {
    let mut translations = vec![String::new(); texts.len()];
    let mut found = vec![false; texts.len()];
    for line in content.lines() {
        let line = line.trim();
        let Some((index_str, text)) = line.split_once(':') else {
            continue;
        };
        let Ok(index) = index_str.trim().parse::<usize>() else {
            continue;
        };
        if index >= 1 && index <= texts.len() && !found[index - 1] {
            translations[index - 1] = text.trim().to_string();
            found[index - 1] = true;
        }
    }
    translations
        .into_iter()
        .zip(texts.iter())
        .map(|(translation, original)| {
            if translation.is_empty() {
                original.clone()
            } else {
                translation
            }
        })
        .collect()
}

pub async fn translate_texts(
    client: &Client,
    model: &str,
    sys_prompt: &str,
    texts: &[String],
) -> Result<Vec<String>> {
    let chat_req = ChatRequest::new(vec![
        ChatMessage::system(format!("{}{}", sys_prompt, NUMBERED_LIST_INSTRUCTIONS)),
        ChatMessage::user(build_numbered_list(texts)),
    ]);

    let response = client.exec_chat(model, chat_req, None).await?;
    let content = response.content_text_as_str().unwrap_or("");
    Ok(parse_numbered_list(content, texts))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_numbered_list_keeps_order() {
        let texts = vec!["one".to_string(), "two".to_string(), "three".to_string()];
        let content = "2: zwei\n1: eins\n3: drei";
        assert_eq!(
            parse_numbered_list(content, &texts),
            vec!["eins", "zwei", "drei"]
        );
    }

    #[test]
    fn test_parse_numbered_list_falls_back_on_missing_line() {
        let texts = vec!["one".to_string(), "two".to_string()];
        let content = "1: eins\nsome unrelated model chatter";
        assert_eq!(parse_numbered_list(content, &texts), vec!["eins", "two"]);
    }

    #[test]
    fn test_build_numbered_list() {
        let texts = vec!["hello".to_string(), "world".to_string()];
        assert_eq!(build_numbered_list(&texts), "1: hello\n2: world");
    }
}
