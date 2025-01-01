
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Serialize)]
struct TranslateRequest<'a> {
    text: Vec<&'a str>,
    source_lang: Option<&'a str>,
    target_lang: &'a str,
}

#[derive(Deserialize)]
struct TranslateResponse {
    translations: Vec<Translation>,
}

#[derive(Deserialize)]
struct Translation {
    text: String,
}

async fn _translate_text(
    base_url: &str,
    path: &str,
    api_key: &str,
    texts: Vec<&str>,
    target_lang: &str,
    source_lang: Option<&str>,
) -> Result<String, Box<dyn Error>> {
    let client = Client::new();
    let request_body = TranslateRequest {
        text: texts,
        source_lang,
        target_lang,
    };
    let response: reqwest::Response = client
        .post(format!("{}{}", base_url, path))
        .header("Content-Type", "application/json")
        .header("Authorization", format!("DeepL-Auth-Key {}", api_key))
        .json(&request_body)
        .send()
        .await?;

    if response.status().is_success() {
        let translate_response: TranslateResponse = response.json().await?;
        if let Some(translation) = translate_response.translations.first() {
            Ok(translation.text.clone())
        } else {
            Err("No translation found".into())
        }
    } else {
        Err(format!("Failed to translate text: {:?}", response.text().await?).into())
    }
}

pub async fn translate_text(
    api_key: &str,
    texts: Vec<&str>,
    target_lang: &str,
    source_lang: Option<&str>,
) -> Result<String, Box<dyn Error>> {
    let base_url =
        std::env::var("DEEPL_API_URL").unwrap_or("https://api-free.deepl.com".to_string());
    let path_url = std::env::var("DEEPL_API_URL_PATH").unwrap_or("/v2/translate".to_string());
    _translate_text(
        &base_url,
        &path_url,
        api_key,
        texts,
        target_lang,
        source_lang,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use tokio;

    #[test]
    fn test_translate_text_sync() {
        // Start a mock server in a synchronous context
        let mut server = Server::new();

        // Create a mock response for the DeepL API
        let _m = server
            .mock("POST", "/v2/translate")
            .match_header("Content-Type", "application/json")
            .match_header("Authorization", "DeepL-Auth-Key test_api_key")
            .match_body(r#"{"text":["Hello World"],"source_lang":"EN","target_lang":"DE"}"#)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"translations":[{"text":"Hallo Welt"}]}"#)
            .create();

        // Set the base URL for the mock server
        let api_key = "test_api_key";
        let texts = vec!["Hello World"];
        let target_lang = "DE";
        let source_lang = Some("EN");

        // Create a runtime to block on the asynchronous function
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            _translate_text(
                server.url().as_str(),
                "/v2/translate",
                api_key,
                texts,
                target_lang,
                source_lang,
            )
            .await
        });

        // Assert the result
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hallo Welt");
    }
}
