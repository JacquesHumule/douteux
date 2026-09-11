use serde::{Deserialize, Serialize};
use worker::KvError;

/// Typed wrapper around the raw `KV` binding. Values are stored as JSON-encoded
/// [`Entry`] blobs keyed by slug.
#[derive(Clone)]
pub struct KvStore {
    store: worker::kv::KvStore,
}

impl KvStore {
    pub fn new(store: worker::kv::KvStore) -> Self {
        Self { store }
    }

    /// Fetch and decode the entry for `key`. `Ok(None)` means the slug is
    /// absent; a stored blob that fails to decode surfaces as `Err`.
    pub async fn get(&self, key: &str) -> Result<Option<Entry>, KvError> {
        self.store.get(key).json().await
    }

    pub async fn put(&self, key: &str, value: &Entry) -> Result<(), KvError> {
        let json = serde_json::to_string(value)?;
        self.store.put(key, json)?.execute().await
    }

    pub async fn delete(&self, key: &str) -> Result<(), KvError> {
        self.store.delete(key).await
    }

    /// Page through every key in the namespace, pairing each slug with its
    /// decoded entry (`None` if the blob is missing or corrupt).
    pub async fn list(&self) -> Result<Vec<(String, Option<Entry>)>, KvError> {
        let mut out = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let mut builder = self.store.list();
            if let Some(c) = cursor.take() {
                builder = builder.cursor(c);
            }
            let page = builder.execute().await?;

            for key in page.keys {
                let entry = self.get(&key.name).await.ok().flatten();
                out.push((key.name, entry));
            }

            if page.list_complete {
                break;
            }
            cursor = page.cursor;
        }

        Ok(out)
    }
}

/// A stored slug target.
#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Entry {
    Url { url: String },
    File { r2_key: String },
}
