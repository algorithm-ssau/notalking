use serde::Deserialize;

#[derive(Clone)]
pub struct HttpEmbeddingProvider {
    client: reqwest::Client,
    base_url: String,
    model: String,
    api_key: Option<String>,
    query_prefix: String,
    document_prefix: String,
}

#[derive(Deserialize)]
struct EmbedResponse {
    data: Vec<EmbedDatum>,
}

#[derive(Deserialize)]
struct EmbedDatum {
    embedding: Vec<f32>,
}

impl HttpEmbeddingProvider {
    pub fn new(
        base_url: String,
        model: String,
        api_key: Option<String>,
        query_prefix: String,
        document_prefix: String,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_owned(),
            model,
            api_key,
            query_prefix,
            document_prefix,
        }
    }

    pub async fn embed_query(&self, text: &str) -> Result<Vec<f32>, String> {
        self.embed_raw(&Self::with_prefix(&self.query_prefix, text))
            .await
    }

    pub async fn embed_document(&self, text: &str) -> Result<Vec<f32>, String> {
        self.embed_raw(&Self::with_prefix(&self.document_prefix, text))
            .await
    }

    fn with_prefix(prefix: &str, text: &str) -> String {
        if prefix.is_empty() {
            text.to_owned()
        } else {
            format!("{prefix}{text}")
        }
    }

    async fn embed_raw(&self, text: &str) -> Result<Vec<f32>, String> {
        let url = format!("{}/embeddings", self.base_url);
        tracing::debug!(
            model = %self.model,
            input_chars = text.chars().count(),
            "embedding request"
        );
        let mut req = self.client.post(&url).json(&serde_json::json!({
            "model": self.model,
            "input": text,
        }));
        if let Some(ref k) = self.api_key {
            req = req.header("Authorization", format!("Bearer {}", k));
        }
        let res = req.send().await.map_err(|e| e.to_string())?;
        let status = res.status();
        if !status.is_success() {
            let body = res.text().await.unwrap_or_default();
            return Err(format!("embedding HTTP {}: {}", status, body));
        }
        let parsed: EmbedResponse = res.json().await.map_err(|e| e.to_string())?;
        let embedding = parsed
            .data
            .into_iter()
            .next()
            .map(|d| d.embedding)
            .ok_or_else(|| "empty embedding data".to_owned())?;
        tracing::debug!(
            model = %self.model,
            dimensions = embedding.len(),
            "embedding response"
        );
        Ok(embedding)
    }
}
