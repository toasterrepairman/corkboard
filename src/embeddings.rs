use embed_anything::{embed_query, embeddings::embed::{Embedder, EmbedData, EmbeddingResult}};
use embed_anything::config::TextEmbedConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;

// Global lazy embedder that gets initialized on first use
static EMBEDDER: Lazy<Arc<Mutex<Option<Embedder>>>> = Lazy::new(|| Arc::new(Mutex::new(None)));

fn ensure_embedder_initialized() -> Result<(), Box<dyn std::error::Error>> {
    let mut embedder_lock = EMBEDDER.lock().unwrap();

    if embedder_lock.is_none() {
        println!("Initializing embedding model (first time only)...");
        // Using Model2Vec for speed - only 8M parameters, very fast
        let embedder = Embedder::from_pretrained_hf(
            "Model2Vec",
            "minishlab/potion-base-8M",
            None,
            None,
            None,
        )?;
        *embedder_lock = Some(embedder);
        println!("Embedding model initialized.");
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnippetEmbedding {
    pub snippet_id: String,
    pub embedding: Vec<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EmbeddingsStore {
    pub embeddings: HashMap<String, Vec<f32>>,
}

impl EmbeddingsStore {
    pub fn new() -> Self {
        Self {
            embeddings: HashMap::new(),
        }
    }

    pub fn load() -> Self {
        let path = Self::get_embeddings_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(store) = serde_json::from_str(&content) {
                    return store;
                }
            }
        }
        Self::new()
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::get_embeddings_path();

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    fn get_embeddings_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("corkboard");
        path.push("embeddings.json");
        path
    }

    pub fn get_embedding(&self, snippet_id: &str) -> Option<&Vec<f32>> {
        self.embeddings.get(snippet_id)
    }

    pub fn set_embedding(&mut self, snippet_id: String, embedding: Vec<f32>) {
        self.embeddings.insert(snippet_id, embedding);
    }

    pub fn remove_embedding(&mut self, snippet_id: &str) {
        self.embeddings.remove(snippet_id);
    }
}

/// Generate an embedding for a text snippet
pub async fn generate_embedding(text: &str) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    // Ensure embedder is initialized (cached after first use)
    ensure_embedder_initialized()?;

    // Create default config
    let config = TextEmbedConfig::default();

    // Get a lock on the embedder and generate embedding
    let embedder_lock = EMBEDDER.lock().unwrap();
    let embedder = embedder_lock.as_ref().unwrap();

    // Generate embedding
    let embeddings = embed_query(&[text], embedder, Some(&config)).await?;

    if let Some(embed_data) = embeddings.first() {
        match &embed_data.embedding {
            EmbeddingResult::DenseVector(embedding) => {
                return Ok(embedding.clone());
            }
            EmbeddingResult::MultiVector(vectors) => {
                // For multi-vector embeddings, we'll take the first vector
                if let Some(first_vec) = vectors.first() {
                    return Ok(first_vec.clone());
                }
            }
        }
    }

    Err("Failed to generate embedding".into())
}

/// Calculate cosine similarity between two embeddings
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        return 0.0;
    }

    dot_product / (magnitude_a * magnitude_b)
}

/// Search for snippets using natural language query
pub async fn search_snippets(
    query: &str,
    store: &EmbeddingsStore,
) -> Result<Vec<(String, f32)>, Box<dyn std::error::Error>> {
    let query_embedding = generate_embedding(query).await?;

    let mut results: Vec<(String, f32)> = store
        .embeddings
        .iter()
        .map(|(id, embedding)| {
            let similarity = cosine_similarity(&query_embedding, embedding);
            (id.clone(), similarity)
        })
        .collect();

    // Sort by similarity (highest first)
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    Ok(results)
}
