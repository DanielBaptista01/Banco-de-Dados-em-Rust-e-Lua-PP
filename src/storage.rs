use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
};

#[derive(Clone, Default)]
pub struct Store {
    entries: Rc<RefCell<HashMap<String, String>>>,
}

impl Store {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, key: String, value: String) {
        self.entries.borrow_mut().insert(key, value);
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.entries.borrow().get(key).cloned()
    }

    pub fn find_key_by_value(&self, value: &str, except_key: Option<&str>) -> Option<String> {
        self.entries.borrow().iter().find_map(|(key, stored_value)| {
            let is_excluded = except_key.is_some_and(|excluded| excluded == key.as_str());
            (stored_value == value && !is_excluded).then(|| key.clone())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Store;

    #[test]
    fn stores_overwrites_and_reads_values() {
        let store = Store::new();
        store.insert("key".to_string(), "first".to_string());
        store.insert("key".to_string(), "second".to_string());
        assert_eq!(store.get("key").as_deref(), Some("second"));
    }

    #[test]
    fn searches_by_value_and_honors_exclusion() {
        let store = Store::new();
        store.insert("key".to_string(), "value".to_string());
        assert_eq!(store.find_key_by_value("value", None).as_deref(), Some("key"));
        assert_eq!(store.find_key_by_value("value", Some("key")), None);
    }
}
