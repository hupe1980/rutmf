//! An in-memory [`ResourceStore`], for tests and for getting started.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard};

use serde_json::Value;

use super::handler::entity_tag;
use super::store::{Matched, Replaced, ResourceStore, Selection, StoreResult};

type Collections = BTreeMap<String, Vec<Value>>;

/// A [`ResourceStore`] holding everything in a `Vec` per collection.
///
/// Cheap to clone — clones share one store, which is what lets a test seed
/// through one handle and read through another. It is what
/// [`MockTmfServer`](crate::mock::MockTmfServer) is built on, and it is a
/// reasonable place to start a real server before there is a database to point
/// at.
///
/// ```
/// use rutmf::server::MemoryStore;
///
/// let store = MemoryStore::new();
/// store.seed("productOffering", serde_json::json!({"id": "7655"}));
///
/// assert_eq!(store.collection("productOffering").len(), 1);
/// ```
#[derive(Debug, Clone, Default)]
pub struct MemoryStore {
    collections: Arc<Mutex<Collections>>,
}

impl MemoryStore {
    /// An empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a resource to `collection` without going through the API.
    ///
    /// Nothing is validated and no id is assigned: this puts the store into a
    /// known state, which is what a test wants.
    pub fn seed(&self, collection: &str, resource: Value) {
        self.lock()
            .entry(collection.to_owned())
            .or_default()
            .push(resource);
    }

    /// Adds several resources to `collection`.
    pub fn seed_all(&self, collection: &str, resources: impl IntoIterator<Item = Value>) {
        self.lock()
            .entry(collection.to_owned())
            .or_default()
            .extend(resources);
    }

    /// Everything held in `collection`, in insertion order.
    #[must_use]
    pub fn collection(&self, collection: &str) -> Vec<Value> {
        self.lock().get(collection).cloned().unwrap_or_default()
    }

    /// Empties every collection.
    pub fn clear(&self) {
        self.lock().clear();
    }

    fn lock(&self) -> MutexGuard<'_, Collections> {
        self.collections
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

fn id_of(resource: &Value) -> Option<&str> {
    resource.get("id").and_then(Value::as_str)
}

#[async_trait::async_trait]
impl ResourceStore for MemoryStore {
    async fn list(&self, collection: &str, selection: &Selection) -> StoreResult<Matched> {
        Ok(selection.apply(self.collection(collection)))
    }

    async fn get(&self, collection: &str, id: &str) -> StoreResult<Option<Value>> {
        Ok(self
            .lock()
            .get(collection)
            .and_then(|items| items.iter().find(|item| id_of(item) == Some(id)).cloned()))
    }

    async fn create(&self, collection: &str, resource: Value) -> StoreResult<Value> {
        self.seed(collection, resource.clone());
        Ok(resource)
    }

    async fn replace(
        &self,
        collection: &str,
        id: &str,
        resource: Value,
    ) -> StoreResult<Option<Value>> {
        let mut held = self.lock();
        let Some(items) = held.get_mut(collection) else {
            return Ok(None);
        };
        let Some(slot) = items.iter_mut().find(|item| id_of(item) == Some(id)) else {
            return Ok(None);
        };
        *slot = resource.clone();
        Ok(Some(resource))
    }

    async fn delete(&self, collection: &str, id: &str) -> StoreResult<bool> {
        let mut held = self.lock();
        let Some(items) = held.get_mut(collection) else {
            return Ok(false);
        };
        let before = items.len();
        items.retain(|item| id_of(item) != Some(id));
        Ok(items.len() != before)
    }

    /// Compares and writes under one lock, so `If-Match` is a real precondition.
    ///
    /// The default implementation reads, compares and writes as three separate
    /// steps; here the mutex is held across all three, which is what closes the
    /// lost-update window rather than merely narrowing it.
    async fn replace_if_unchanged(
        &self,
        collection: &str,
        id: &str,
        resource: Value,
        expected_tag: &str,
    ) -> StoreResult<Replaced> {
        let mut held = self.lock();
        let Some(slot) = held
            .get_mut(collection)
            .and_then(|items| items.iter_mut().find(|item| id_of(item) == Some(id)))
        else {
            return Ok(Replaced::Missing);
        };
        if entity_tag(slot) != expected_tag {
            return Ok(Replaced::Stale);
        }
        *slot = resource.clone();
        Ok(Replaced::Updated(resource))
    }

    /// The delete half of [`replace_if_unchanged`](Self::replace_if_unchanged),
    /// likewise under one lock.
    async fn delete_if_unchanged(
        &self,
        collection: &str,
        id: &str,
        expected_tag: &str,
    ) -> StoreResult<Replaced> {
        let mut held = self.lock();
        let Some(items) = held.get_mut(collection) else {
            return Ok(Replaced::Missing);
        };
        let Some(at) = items.iter().position(|item| id_of(item) == Some(id)) else {
            return Ok(Replaced::Missing);
        };
        if entity_tag(&items[at]) != expected_tag {
            return Ok(Replaced::Stale);
        }
        Ok(Replaced::Updated(items.remove(at)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn clones_share_one_store() {
        let store = MemoryStore::new();
        let other = store.clone();
        other.seed("thing", serde_json::json!({"id": "1"}));

        assert_eq!(store.collection("thing").len(), 1);
        assert!(store.get("thing", "1").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn replacing_an_absent_resource_reports_it_rather_than_inserting() {
        let store = MemoryStore::new();
        let outcome = store
            .replace("thing", "1", serde_json::json!({"id": "1"}))
            .await
            .unwrap();

        assert_eq!(outcome, None);
        assert!(store.collection("thing").is_empty());
    }

    #[tokio::test]
    async fn a_conditional_write_refuses_a_resource_that_moved_underneath_it() {
        // The lost update this closes: two clients read the same resource, both
        // pass the `If-Match` check, and the second write silently discards the
        // first — with `200` to both. The tag is compared and the write happens
        // under one lock, so the second attempt is refused instead.
        let store = MemoryStore::new();
        store.seed("thing", serde_json::json!({"id": "1", "name": "original"}));

        let read = store.get("thing", "1").await.unwrap().unwrap();
        let tag = entity_tag(&read);

        // Somebody else gets there first.
        let winner = store
            .replace_if_unchanged(
                "thing",
                "1",
                serde_json::json!({"id": "1", "name": "first"}),
                &tag,
            )
            .await
            .unwrap();
        assert!(matches!(winner, Replaced::Updated(_)));

        // The second write is holding a tag that no longer describes anything.
        let loser = store
            .replace_if_unchanged(
                "thing",
                "1",
                serde_json::json!({"id": "1", "name": "second"}),
                &tag,
            )
            .await
            .unwrap();
        assert_eq!(loser, Replaced::Stale, "the first write must not be lost");
        assert_eq!(store.collection("thing")[0]["name"], "first");

        // The same holds for a delete.
        assert_eq!(
            store.delete_if_unchanged("thing", "1", &tag).await.unwrap(),
            Replaced::Stale,
        );
        assert_eq!(store.collection("thing").len(), 1);
    }

    #[tokio::test]
    async fn a_conditional_write_reports_an_absent_resource_apart_from_a_stale_one() {
        // `404` and `412` are different answers and a client acts differently
        // on each, so the store must not collapse them.
        let store = MemoryStore::new();
        assert_eq!(
            store
                .replace_if_unchanged("thing", "1", serde_json::json!({"id": "1"}), "\"any\"")
                .await
                .unwrap(),
            Replaced::Missing,
        );
        assert_eq!(
            store
                .delete_if_unchanged("thing", "1", "\"any\"")
                .await
                .unwrap(),
            Replaced::Missing,
        );
    }

    #[tokio::test]
    async fn deleting_reports_whether_there_was_anything_to_delete() {
        let store = MemoryStore::new();
        store.seed("thing", serde_json::json!({"id": "1"}));

        assert!(store.delete("thing", "1").await.unwrap());
        assert!(!store.delete("thing", "1").await.unwrap());
    }
}
