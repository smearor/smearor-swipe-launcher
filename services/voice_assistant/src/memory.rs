use crate::embedding_engine::EmbeddingEngine;
use crate::embedding_engine::SharedEmbeddingEngine;
use crate::embedding_engine::cosine_similarity;
use rusqlite::Connection;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use tracing::debug;

/// Errors that can occur during memory operations.
#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    /// SQLite database error.
    #[error("Database error: {0}")]
    Database(String),
    /// Embedding generation failed.
    #[error("Embedding failed: {0}")]
    EmbeddingFailed(String),
    /// Model loading failed.
    #[error("Model load failed: {0}")]
    ModelLoad(String),
    /// Database lock poisoned.
    #[error("Database lock poisoned")]
    DbLocked,
}

/// Represents the state of a controllable entity (e.g., a smart home device).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityState {
    /// Human-readable name of the entity (e.g., "Ventilator").
    pub name: String,
    /// Current state (e.g., "on", "off", "open", "closed").
    pub state: String,
    /// Tool name that controls this entity (e.g., "button_shelly_fan_button").
    pub tool: String,
    /// Last action performed (e.g., "click", "longpress").
    pub last_action: String,
    /// ISO 8601 timestamp of the last state change.
    pub last_changed: String,
}

/// In-memory store of entity states, keyed by entity identifier.
pub type EntityStore = Arc<RwLock<HashMap<String, EntityState>>>;

/// Category of a stored fact for organizational purposes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FactCategory {
    /// A general fact or piece of information.
    Fact,
    /// A user preference.
    Preference,
    /// A recurring habit or pattern.
    Habit,
}

impl Default for FactCategory {
    fn default() -> Self {
        Self::Fact
    }
}

impl std::fmt::Display for FactCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fact => write!(f, "fact"),
            Self::Preference => write!(f, "preference"),
            Self::Habit => write!(f, "habit"),
        }
    }
}

impl std::str::FromStr for FactCategory {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "fact" => Ok(Self::Fact),
            "preference" => Ok(Self::Preference),
            "habit" => Ok(Self::Habit),
            other => Err(format!("Unknown fact category: {other}")),
        }
    }
}

/// A stored fact with its semantic embedding for long-term memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredFact {
    /// Unique identifier (UUID).
    pub id: String,
    /// Short key for the fact (e.g., "user_name").
    pub key: String,
    /// The fact content.
    pub value: String,
    /// Category of the fact.
    pub category: FactCategory,
    /// Semantic embedding vector.
    #[serde(skip)]
    pub embedding: Vec<f32>,
    /// ISO 8601 creation timestamp.
    pub created_at: String,
    /// ISO 8601 last accessed timestamp.
    pub last_accessed: String,
    /// Number of times this fact has been recalled.
    pub access_count: u32,
}

/// Semantic memory backed by fastembed embeddings and SQLite persistence.
/// Supports GPU acceleration via ONNX Runtime execution providers and
/// caches embeddings with moka to avoid redundant computation.
pub struct SemanticMemory {
    embedding_engine: Option<SharedEmbeddingEngine>,
    vectors: Vec<(Vec<f32>, String)>,
    db: Arc<Mutex<Connection>>,
}

impl SemanticMemory {
    /// Creates an uninitialized placeholder SemanticMemory.
    /// Used when the real initialization (which loads the embedding model) is deferred.
    /// The placeholder has an empty vector store and a no-op database connection.
    pub fn uninit() -> Self {
        let db = Connection::open_in_memory()
            .map_err(|e| MemoryError::Database(e.to_string()))
            .expect("in-memory SQLite should always succeed");
        init_schema(&db).expect("schema init should succeed");
        Self {
            embedding_engine: None,
            vectors: Vec::new(),
            db: Arc::new(Mutex::new(db)),
        }
    }

    /// Initializes the semantic memory: loads the embedding model, opens SQLite,
    /// creates schema, and loads existing vectors from the database.
    pub fn new(db_path: &str, embedding_model: &str) -> Result<Self, MemoryError> {
        let embedding_engine = EmbeddingEngine::load(embedding_model)
            .or_else(|e| {
                debug!("Semantic memory: failed to load embedding model: {e}, trying fallback");
                EmbeddingEngine::fallback()
            })
            .map_err(|e| MemoryError::ModelLoad(e.to_string()))?;
        let embedding_engine = Arc::new(embedding_engine);
        debug!("Semantic memory: embedding model loaded");

        let expanded_path = shellexpand::tilde(db_path).into_owned();
        if let Some(parent) = std::path::Path::new(&expanded_path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let db = Connection::open(&expanded_path).map_err(|e| MemoryError::Database(e.to_string()))?;
        init_schema(&db)?;

        let db = Arc::new(Mutex::new(db));
        let mut memory = Self {
            embedding_engine: Some(embedding_engine),
            vectors: Vec::new(),
            db,
        };
        memory.load_vectors_from_db()?;
        debug!("Semantic memory: initialized with {} vectors", memory.vectors.len());
        Ok(memory)
    }

    /// Returns a reference to the shared embedding engine.
    /// Allows other components (ToolRouter, CatalogRouter) to reuse the same model.
    /// Returns None when SemanticMemory is in its uninit placeholder state.
    pub fn embedding_engine(&self) -> Option<&SharedEmbeddingEngine> {
        self.embedding_engine.as_ref()
    }

    /// Stores a fact with its semantic embedding.
    /// Uses the embedding cache to avoid re-embedding identical text.
    pub fn store(&mut self, key: &str, value: &str, category: FactCategory) -> Result<String, MemoryError> {
        let id = uuid::Uuid::new_v4().to_string();
        let embedding = self.embed_single(value)?;

        let fact = StoredFact {
            id: id.clone(),
            key: key.to_string(),
            value: value.to_string(),
            category,
            embedding: embedding.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            last_accessed: chrono::Utc::now().to_rfc3339(),
            access_count: 0,
        };
        self.persist_fact(&fact)?;
        self.vectors.push((embedding, id.clone()));
        debug!("Semantic memory: stored fact '{key}' with id {id}");
        Ok(id)
    }

    /// Stores multiple facts in a single batch.
    /// Embeds all values in one call to leverage batch processing efficiency.
    /// Each fact gets its own key but shares the category.
    pub fn store_batch(&mut self, facts: &[(String, String, FactCategory)]) -> Result<Vec<String>, MemoryError> {
        if facts.is_empty() {
            return Ok(Vec::new());
        }

        let texts: Vec<String> = facts.iter().map(|(_, value, _)| value.clone()).collect();
        let embeddings = self.embed_batch(&texts)?;

        let mut ids = Vec::with_capacity(facts.len());
        for ((key, value, category), embedding) in facts.iter().zip(embeddings.iter()) {
            let id = uuid::Uuid::new_v4().to_string();
            let fact = StoredFact {
                id: id.clone(),
                key: key.clone(),
                value: value.clone(),
                category: category.clone(),
                embedding: embedding.clone(),
                created_at: chrono::Utc::now().to_rfc3339(),
                last_accessed: chrono::Utc::now().to_rfc3339(),
                access_count: 0,
            };
            self.persist_fact(&fact)?;
            self.vectors.push((embedding.clone(), id.clone()));
            ids.push(id.clone());
            debug!("Semantic memory: batch-stored fact '{key}' with id {id}");
        }
        Ok(ids)
    }

    /// Recalls facts semantically related to the query.
    /// Returns the top N facts by cosine similarity.
    /// Uses the embedding cache to avoid re-embedding identical queries.
    pub fn recall(&mut self, query: &str, top_n: usize) -> Result<Vec<StoredFact>, MemoryError> {
        if self.vectors.is_empty() {
            return Ok(Vec::new());
        }
        let query_embedding = self.embed_single(query)?;

        let mut scored: Vec<(f32, &str)> = self
            .vectors
            .iter()
            .map(|(emb, id)| (cosine_similarity(&query_embedding, emb), id.as_str()))
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let top_ids: Vec<&str> = scored.iter().take(top_n).map(|(_, id)| *id).collect();
        let facts = self.load_facts_by_ids(&top_ids)?;
        for fact in &facts {
            self.touch_fact(&fact.id)?;
        }
        Ok(facts)
    }

    /// Embeds a single text string, using the embedding engine cache.
    fn embed_single(&self, text: &str) -> Result<Vec<f32>, MemoryError> {
        self.embedding_engine
            .as_ref()
            .ok_or(MemoryError::ModelLoad("embedding engine not initialized".to_string()))?
            .embed_single(text)
            .map_err(|e| MemoryError::EmbeddingFailed(e.to_string()))
    }

    /// Embeds multiple texts in a single batch call for efficiency.
    fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, MemoryError> {
        self.embedding_engine
            .as_ref()
            .ok_or(MemoryError::ModelLoad("embedding engine not initialized".to_string()))?
            .embed_batch(texts)
            .map_err(|e| MemoryError::EmbeddingFailed(e.to_string()))
    }

    /// Lists all stored fact keys, optionally filtered by category.
    pub fn list_keys(&self, category: Option<&FactCategory>) -> Result<Vec<(String, String)>, MemoryError> {
        let db = self.db.lock().map_err(|_| MemoryError::DbLocked)?;
        let mut sql = "SELECT key, category FROM facts".to_string();
        if category.is_some() {
            sql.push_str(" WHERE category = ?1");
        }
        sql.push_str(" ORDER BY key");
        let mut stmt = db.prepare(&sql).map_err(|e| MemoryError::Database(e.to_string()))?;
        let cat_param = category.map(|c| c.to_string());
        let rows = stmt
            .query_map(rusqlite::params_from_iter(cat_param.iter()), |row| {
                let key: String = row.get(0)?;
                let cat: String = row.get(1)?;
                Ok((key, cat))
            })
            .map_err(|e| MemoryError::Database(e.to_string()))?;
        let mut result = Vec::new();
        for row in rows {
            if let Ok((key, cat)) = row {
                result.push((key, cat));
            }
        }
        Ok(result)
    }

    /// Deletes a fact by key.
    pub fn forget(&self, key: &str) -> Result<(), MemoryError> {
        let db = self.db.lock().map_err(|_| MemoryError::DbLocked)?;
        db.execute("DELETE FROM facts WHERE key = ?1", rusqlite::params![key])
            .map_err(|e| MemoryError::Database(e.to_string()))?;
        debug!("Semantic memory: forgot fact '{key}'");
        Ok(())
    }

    /// Stores an entity state change in the entity_history table.
    pub fn write_entity_history(&self, state: &EntityState) -> Result<(), MemoryError> {
        let db = self.db.lock().map_err(|_| MemoryError::DbLocked)?;
        db.execute(
            "INSERT INTO entity_history (entity, state, action, tool, timestamp) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![state.name, state.state, state.last_action, state.tool, state.last_changed],
        )
        .map_err(|e| MemoryError::Database(e.to_string()))?;
        Ok(())
    }

    /// Reconstructs the entity store from the latest entity_history entries.
    pub fn reconstruct_entity_store(&self) -> Result<HashMap<String, EntityState>, MemoryError> {
        let db = self.db.lock().map_err(|_| MemoryError::DbLocked)?;
        let mut stmt = db
            .prepare(
                "SELECT entity, state, action, tool, timestamp FROM entity_history \
                 WHERE id IN (SELECT MAX(id) FROM entity_history GROUP BY entity)",
            )
            .map_err(|e| MemoryError::Database(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| {
                let name: String = row.get(0)?;
                let state: String = row.get(1)?;
                let action: String = row.get(2)?;
                let tool: String = row.get(3)?;
                let timestamp: String = row.get(4)?;
                Ok(EntityState {
                    name,
                    state,
                    tool,
                    last_action: action,
                    last_changed: timestamp,
                })
            })
            .map_err(|e| MemoryError::Database(e.to_string()))?;
        let mut store = HashMap::new();
        for row in rows {
            if let Ok(entity) = row {
                store.insert(entity.tool.clone(), entity);
            }
        }
        debug!("Semantic memory: reconstructed {} entity states from history", store.len());
        Ok(store)
    }

    fn persist_fact(&self, fact: &StoredFact) -> Result<(), MemoryError> {
        let db = self.db.lock().map_err(|_| MemoryError::DbLocked)?;
        let embedding_bytes = serialize_embedding(&fact.embedding);
        db.execute(
            "INSERT INTO facts (id, key, value, category, embedding, created_at, last_accessed, access_count) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                fact.id,
                fact.key,
                fact.value,
                fact.category.to_string(),
                embedding_bytes,
                fact.created_at,
                fact.last_accessed,
                fact.access_count,
            ],
        )
        .map_err(|e| MemoryError::Database(e.to_string()))?;
        Ok(())
    }

    fn load_facts_by_ids(&self, ids: &[&str]) -> Result<Vec<StoredFact>, MemoryError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let db = self.db.lock().map_err(|_| MemoryError::DbLocked)?;
        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT id, key, value, category, embedding, created_at, last_accessed, access_count \
             FROM facts WHERE id IN ({placeholders})"
        );
        let mut stmt = db.prepare(&sql).map_err(|e| MemoryError::Database(e.to_string()))?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(ids.iter().copied()), |row| {
                let id: String = row.get(0)?;
                let key: String = row.get(1)?;
                let value: String = row.get(2)?;
                let category_str: String = row.get(3)?;
                let embedding_blob: Vec<u8> = row.get(4)?;
                let created_at: String = row.get(5)?;
                let last_accessed: String = row.get(6)?;
                let access_count: u32 = row.get(7)?;
                let category = category_str.parse().unwrap_or(FactCategory::Fact);
                let embedding = deserialize_embedding(&embedding_blob);
                Ok(StoredFact {
                    id,
                    key,
                    value,
                    category,
                    embedding,
                    created_at,
                    last_accessed,
                    access_count,
                })
            })
            .map_err(|e| MemoryError::Database(e.to_string()))?;
        let mut facts = Vec::new();
        for row in rows {
            if let Ok(fact) = row {
                facts.push(fact);
            }
        }
        Ok(facts)
    }

    fn touch_fact(&self, id: &str) -> Result<(), MemoryError> {
        let db = self.db.lock().map_err(|_| MemoryError::DbLocked)?;
        db.execute(
            "UPDATE facts SET last_accessed = ?1, access_count = access_count + 1 WHERE id = ?2",
            rusqlite::params![chrono::Utc::now().to_rfc3339(), id],
        )
        .map_err(|e| MemoryError::Database(e.to_string()))?;
        Ok(())
    }

    fn load_vectors_from_db(&mut self) -> Result<(), MemoryError> {
        let db = self.db.lock().map_err(|_| MemoryError::DbLocked)?;
        let mut stmt = db
            .prepare("SELECT id, embedding FROM facts")
            .map_err(|e| MemoryError::Database(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let embedding_blob: Vec<u8> = row.get(1)?;
                Ok((id, deserialize_embedding(&embedding_blob)))
            })
            .map_err(|e| MemoryError::Database(e.to_string()))?;
        for row in rows {
            if let Ok((id, embedding)) = row {
                self.vectors.push((embedding, id));
            }
        }
        Ok(())
    }
}

/// Shared semantic memory type used by the service.
pub type SharedSemanticMemory = Arc<RwLock<SemanticMemory>>;

/// Extracts entity state from a tool call and its arguments.
pub fn extract_entity_state(tool_name: &str, arguments: &serde_json::Value) -> Option<EntityState> {
    if let Some(plugin_id) = tool_name.strip_prefix("button_") {
        let action = arguments.get("action").and_then(|v| v.as_str()).unwrap_or("click");
        let state = match action {
            "click" => "on",
            "longpress" => "off",
            "swipe_up" => "increasing",
            "swipe_down" => "decreasing",
            _ => return None,
        };
        return Some(EntityState {
            name: plugin_id.replace('_', " "),
            state: state.to_string(),
            tool: tool_name.to_string(),
            last_action: action.to_string(),
            last_changed: chrono::Utc::now().to_rfc3339(),
        });
    }

    if tool_name == "app_launcher_exec" {
        let app = arguments.get("desktop_file").and_then(|v| v.as_str()).unwrap_or("unknown");
        let app_name = std::path::Path::new(app).file_stem().and_then(|s| s.to_str()).unwrap_or(app).to_string();
        return Some(EntityState {
            name: app_name,
            state: "running".to_string(),
            tool: tool_name.to_string(),
            last_action: "exec".to_string(),
            last_changed: chrono::Utc::now().to_rfc3339(),
        });
    }

    if tool_name == "app_launcher_terminate" {
        let app = arguments.get("desktop_file").and_then(|v| v.as_str()).unwrap_or("unknown");
        let app_name = std::path::Path::new(app).file_stem().and_then(|s| s.to_str()).unwrap_or(app).to_string();
        return Some(EntityState {
            name: app_name,
            state: "stopped".to_string(),
            tool: tool_name.to_string(),
            last_action: "terminate".to_string(),
            last_changed: chrono::Utc::now().to_rfc3339(),
        });
    }

    if tool_name == "audio_set_volume" {
        let volume = arguments.get("volume").and_then(|v| v.as_f64()).unwrap_or(0.0);
        return Some(EntityState {
            name: "audio_volume".to_string(),
            state: format!("{:.0}%", volume * 100.0),
            tool: tool_name.to_string(),
            last_action: "set_volume".to_string(),
            last_changed: chrono::Utc::now().to_rfc3339(),
        });
    }

    if tool_name == "mpris_play" || tool_name == "mpris_pause" || tool_name == "mpris_toggle_play_pause" {
        let state = match tool_name {
            "mpris_play" => "playing",
            "mpris_pause" => "paused",
            _ => "toggled",
        };
        return Some(EntityState {
            name: "media_player".to_string(),
            state: state.to_string(),
            tool: tool_name.to_string(),
            last_action: tool_name.to_string(),
            last_changed: chrono::Utc::now().to_rfc3339(),
        });
    }

    None
}

// cosine_similarity is now in crate::embedding_engine

/// Serializes an embedding to little-endian f32 bytes for SQLite BLOB storage.
fn serialize_embedding(emb: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(emb.len() * 4);
    for &v in emb {
        bytes.extend_from_slice(&v.to_le_bytes());
    }
    bytes
}

/// Deserializes an embedding from little-endian f32 bytes.
fn deserialize_embedding(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

/// Initializes the SQLite schema for facts and entity history.
fn init_schema(db: &Connection) -> Result<(), MemoryError> {
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS facts (
            id TEXT PRIMARY KEY,
            key TEXT NOT NULL,
            value TEXT NOT NULL,
            category TEXT NOT NULL DEFAULT 'fact',
            embedding BLOB NOT NULL,
            created_at TEXT NOT NULL,
            last_accessed TEXT NOT NULL,
            access_count INTEGER DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS entity_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            entity TEXT NOT NULL,
            state TEXT NOT NULL,
            action TEXT NOT NULL,
            tool TEXT NOT NULL,
            timestamp TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_facts_key ON facts(key);
        CREATE INDEX IF NOT EXISTS idx_facts_category ON facts(category);
        CREATE INDEX IF NOT EXISTS idx_entity_history_entity ON entity_history(entity);
        CREATE INDEX IF NOT EXISTS idx_entity_history_timestamp ON entity_history(timestamp);
        ",
    )
    .map_err(|e| MemoryError::Database(e.to_string()))?;
    Ok(())
}
