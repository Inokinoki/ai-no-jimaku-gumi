use genai::chat::{ChatMessage, ChatRequest};
use genai::Client;
use std::error::Error;

pub async fn translate_text(
    client: &Client,
    model: &str,
    sys_prompt: &str,
    texts: Vec<&str>,
) -> Result<String, Box<dyn Error>> {
    let chat_req = ChatRequest::new(vec![
        ChatMessage::system(sys_prompt),
        ChatMessage::user(texts.join(" ")),
    ]);

    let response = client.exec_chat(model, chat_req, None).await?;
    Ok(response.content_text_as_str().unwrap_or("").to_string())
}
