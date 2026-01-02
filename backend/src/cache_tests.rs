#[cfg(test)]
mod cache_tests {
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_hashmap_basic_operations() {
        let mut cache: HashMap<String, String> = HashMap::new();
        
        cache.insert("key1".to_string(), "value1".to_string());
        cache.insert("key2".to_string(), "value2".to_string());
        
        assert_eq!(cache.get("key1"), Some(&"value1".to_string()));
        assert_eq!(cache.get("key2"), Some(&"value2".to_string()));
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_hashmap_removal() {
        let mut cache: HashMap<String, String> = HashMap::new();
        
        cache.insert("key1".to_string(), "value1".to_string());
        cache.insert("key2".to_string(), "value2".to_string());
        
        let removed = cache.remove("key1");
        assert_eq!(removed, Some("value1".to_string()));
        assert_eq!(cache.len(), 1);
        assert!(cache.get("key1").is_none());
    }

    #[test]
    fn test_hashmap_thread_safety() {
        let cache = Arc::new(std::sync::Mutex::new(HashMap::new()));
        let mut handles = vec![];

        for i in 0..10 {
            let cache_clone = Arc::clone(&cache);
            let handle = thread::spawn(move || {
                let mut cache = cache_clone.lock().unwrap();
                cache.insert(format!("key_{}", i), format!("value_{}", i));
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let cache = cache.lock().unwrap();
        assert_eq!(cache.len(), 10);
    }

    #[test]
    fn test_hashmap_contains_key() {
        let mut cache: HashMap<String, String> = HashMap::new();
        
        cache.insert("key1".to_string(), "value1".to_string());
        
        assert!(cache.contains_key("key1"));
        assert!(!cache.contains_key("key2"));
    }
}