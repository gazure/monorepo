use std::collections::{BTreeMap, btree_map::Entry};

pub struct MultiMap<K, V> {
    inner: BTreeMap<K, Vec<V>>,
}

impl<K: Ord, V> Default for MultiMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Ord, V> MultiMap<K, V> {
    pub fn new() -> Self {
        Self { inner: BTreeMap::new() }
    }

    pub fn insert(&mut self, key: K, value: V) {
        self.inner.entry(key).or_default().push(value);
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.get_all(key)?.first()
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.get_all_mut(key)?.first_mut()
    }

    pub fn get_all(&self, key: &K) -> Option<&[V]> {
        self.inner.get(key).map(std::convert::AsRef::as_ref)
    }

    pub fn get_all_mut(&mut self, key: &K) -> Option<&mut Vec<V>> {
        self.inner.get_mut(key)
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }

    pub fn remove_all(&mut self, key: &K) -> Option<Vec<V>> {
        self.inner.remove(key)
    }

    pub fn pop(&mut self, key: &K) -> Option<V> {
        let values = self.inner.get_mut(key)?;
        let popped = values.pop();
        if values.is_empty() {
            self.inner.remove(key);
        }
        popped
    }

    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.inner.keys()
    }

    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.inner.values().flat_map(|v| v.iter())
    }

    pub fn vec_values(&self) -> impl Iterator<Item = &Vec<V>> {
        self.inner.values()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.inner.iter().flat_map(|(k, v)| v.iter().map(move |v| (k, v)))
    }

    pub fn vec_iter(&self) -> impl Iterator<Item = (&K, &Vec<V>)> {
        self.inner.iter()
    }

    pub fn vec_iter_mut(&mut self) -> impl Iterator<Item = (&K, &mut Vec<V>)> {
        self.inner.iter_mut()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&K, &mut V)> {
        self.inner
            .iter_mut()
            .flat_map(|(k, v)| v.iter_mut().map(move |v| (k, v)))
    }

    pub fn entry(&mut self, key: K) -> Entry<'_, K, Vec<V>> {
        self.inner.entry(key)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_new() {
        let multimap: MultiMap<String, i32> = MultiMap::new();
        assert_eq!(multimap.len(), 0);
        assert_eq!(multimap.get(&"key".to_string()), None);
    }

    #[test]
    fn test_insert_single_value() {
        let mut multimap = MultiMap::new();
        multimap.insert("key1", 10);

        assert_eq!(multimap.get(&"key1"), Some(&10));
        assert_eq!(multimap.len(), 1);
    }

    #[test]
    fn test_insert_multiple_values_same_key() {
        let mut multimap = MultiMap::new();
        multimap.insert("key1", 10);
        multimap.insert("key1", 20);
        multimap.insert("key1", 30);

        assert_eq!(multimap.get(&"key1"), Some(&10)); // First value
        assert_eq!(multimap.get_all(&"key1"), Some(&[10, 20, 30][..]));
        assert_eq!(multimap.len(), 1); // Still one key
    }

    #[test]
    fn test_insert_multiple_keys() {
        let mut multimap = MultiMap::new();
        multimap.insert("key1", 10);
        multimap.insert("key2", 20);
        multimap.insert("key3", 30);

        assert_eq!(multimap.get(&"key1"), Some(&10));
        assert_eq!(multimap.get(&"key2"), Some(&20));
        assert_eq!(multimap.get(&"key3"), Some(&30));
        assert_eq!(multimap.len(), 3);
    }

    #[test]
    fn test_get_nonexistent_key() {
        let multimap: MultiMap<&str, i32> = MultiMap::new();
        assert_eq!(multimap.get(&"nonexistent"), None);
        assert_eq!(multimap.get_all(&"nonexistent"), None);
    }

    #[test]
    fn test_get_mut() {
        let mut multimap = MultiMap::new();
        multimap.insert("key1", 10);
        multimap.insert("key1", 20);

        if let Some(value) = multimap.get_mut(&"key1") {
            *value = 100;
        }

        assert_eq!(multimap.get(&"key1"), Some(&100));
        assert_eq!(multimap.get_all(&"key1"), Some(&[100, 20][..]));
    }

    #[test]
    fn test_get_all() {
        let mut multimap = MultiMap::new();
        multimap.insert("key1", 1);
        multimap.insert("key1", 2);
        multimap.insert("key1", 3);
        multimap.insert("key2", 4);

        assert_eq!(multimap.get_all(&"key1"), Some(&[1, 2, 3][..]));
        assert_eq!(multimap.get_all(&"key2"), Some(&[4][..]));
        assert_eq!(multimap.get_all(&"key3"), None);
    }

    #[test]
    fn test_get_all_mut() {
        let mut multimap = MultiMap::new();
        multimap.insert("key1", 1);
        multimap.insert("key1", 2);
        multimap.insert("key1", 3);

        if let Some(values) = multimap.get_all_mut(&"key1") {
            values[0] = 10;
            values[1] = 20;
            values.push(40);
        }

        assert_eq!(multimap.get_all(&"key1"), Some(&[10, 20, 3, 40][..]));
    }

    #[test]
    fn test_len() {
        let mut multimap = MultiMap::new();
        assert_eq!(multimap.len(), 0);

        multimap.insert("key1", 1);
        assert_eq!(multimap.len(), 1);

        multimap.insert("key1", 2); // Same key
        assert_eq!(multimap.len(), 1);

        multimap.insert("key2", 3);
        assert_eq!(multimap.len(), 2);
    }

    #[test]
    fn test_is_empty() {
        let mut multimap = MultiMap::new();
        assert!(multimap.is_empty());

        multimap.insert("key1", 1);
        assert!(!multimap.is_empty());

        multimap.insert("key1", 2);
        assert!(!multimap.is_empty());

        multimap.insert("key2", 3);
        assert!(!multimap.is_empty());

        multimap.remove_all(&"key1");
        assert!(!multimap.is_empty());

        multimap.remove_all(&"key2");
        assert!(multimap.is_empty());

        multimap.insert("key3", 4);
        assert!(!multimap.is_empty());

        multimap.clear();
        assert!(multimap.is_empty());
    }

    #[test]
    fn test_clear() {
        let mut multimap = MultiMap::new();
        multimap.insert("key1", 1);
        multimap.insert("key1", 2);
        multimap.insert("key2", 3);

        assert_eq!(multimap.len(), 2);

        multimap.clear();
        assert_eq!(multimap.len(), 0);
        assert_eq!(multimap.get(&"key1"), None);
        assert_eq!(multimap.get(&"key2"), None);
    }

    #[test]
    fn test_remove_all() {
        let mut multimap = MultiMap::new();
        multimap.insert("key1", 1);
        multimap.insert("key1", 2);
        multimap.insert("key1", 3);
        multimap.insert("key2", 4);

        let removed = multimap.remove_all(&"key1");
        assert_eq!(removed, Some(vec![1, 2, 3]));
        assert_eq!(multimap.get(&"key1"), None);
        assert_eq!(multimap.get(&"key2"), Some(&4));
        assert_eq!(multimap.len(), 1);

        let removed_none = multimap.remove_all(&"key3");
        assert_eq!(removed_none, None);
    }

    #[test]
    fn test_pop() {
        let mut multimap = MultiMap::new();
        multimap.insert("key1", 1);
        multimap.insert("key1", 2);
        multimap.insert("key1", 3);

        assert_eq!(multimap.pop(&"key1"), Some(3));
        assert_eq!(multimap.get_all(&"key1"), Some(&[1, 2][..]));

        assert_eq!(multimap.pop(&"key1"), Some(2));
        assert_eq!(multimap.get_all(&"key1"), Some(&[1][..]));

        assert_eq!(multimap.pop(&"key1"), Some(1));
        assert_eq!(multimap.get_all(&"key1"), None);
        assert_eq!(multimap.len(), 0);

        assert_eq!(multimap.pop(&"key1"), None);
    }

    #[test]
    fn test_pop_nonexistent_key() {
        let mut multimap: MultiMap<&str, i32> = MultiMap::new();
        assert_eq!(multimap.pop(&"nonexistent"), None);
    }

    #[test]
    fn test_keys() {
        let mut multimap = MultiMap::new();
        multimap.insert("b", 1);
        multimap.insert("a", 2);
        multimap.insert("c", 3);
        multimap.insert("a", 4);

        let keys: Vec<_> = multimap.keys().collect();
        assert_eq!(keys, vec![&"a", &"b", &"c"]); // BTreeMap keeps keys sorted
    }

    #[test]
    fn test_values() {
        let mut multimap = MultiMap::new();
        multimap.insert("key1", 1);
        multimap.insert("key1", 2);
        multimap.insert("key2", 3);
        multimap.insert("key2", 4);

        let values: Vec<_> = multimap.values().copied().collect();
        assert_eq!(values, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_vec_values() {
        let mut multimap = MultiMap::new();
        multimap.insert("key1", 1);
        multimap.insert("key1", 2);
        multimap.insert("key2", 3);
        multimap.insert("key2", 4);

        let vec_values: Vec<_> = multimap.vec_values().collect();
        assert_eq!(vec_values[0], &vec![1, 2]);
        assert_eq!(vec_values[1], &vec![3, 4]);
    }

    #[test]
    fn test_iter() {
        let mut multimap = MultiMap::new();
        multimap.insert("a", 1);
        multimap.insert("a", 2);
        multimap.insert("b", 3);

        let pairs: Vec<_> = multimap.iter().map(|(k, v)| (*k, *v)).collect();
        assert_eq!(pairs, vec![("a", 1), ("a", 2), ("b", 3)]);
    }

    #[test]
    fn test_vec_iter() {
        let mut multimap = MultiMap::new();
        multimap.insert("a", 1);
        multimap.insert("a", 2);
        multimap.insert("b", 3);

        let vec_pairs: Vec<_> = multimap.vec_iter().collect();
        assert_eq!(vec_pairs[0].0, &"a");
        assert_eq!(vec_pairs[0].1, &vec![1, 2]);
        assert_eq!(vec_pairs[1].0, &"b");
        assert_eq!(vec_pairs[1].1, &vec![3]);
    }

    #[test]
    fn test_vec_iter_mut() {
        let mut multimap = MultiMap::new();
        multimap.insert("a", 1);
        multimap.insert("a", 2);
        multimap.insert("b", 3);

        for (_, values) in multimap.vec_iter_mut() {
            for value in values.iter_mut() {
                *value *= 10;
            }
        }

        assert_eq!(multimap.get_all(&"a"), Some(&[10, 20][..]));
        assert_eq!(multimap.get_all(&"b"), Some(&[30][..]));
    }

    #[test]
    fn test_iter_mut() {
        let mut multimap = MultiMap::new();
        multimap.insert("a", 1);
        multimap.insert("a", 2);
        multimap.insert("b", 3);

        for (_, value) in multimap.iter_mut() {
            *value *= 10;
        }

        assert_eq!(multimap.get_all(&"a"), Some(&[10, 20][..]));
        assert_eq!(multimap.get_all(&"b"), Some(&[30][..]));
    }

    #[test]
    fn test_complex_scenario() {
        let mut multimap = MultiMap::new();

        // Insert multiple values for multiple keys
        multimap.insert("fruits", "apple");
        multimap.insert("fruits", "banana");
        multimap.insert("fruits", "cherry");
        multimap.insert("vegetables", "carrot");
        multimap.insert("vegetables", "broccoli");
        multimap.insert("grains", "rice");

        // Test various operations
        assert_eq!(multimap.len(), 3);
        assert_eq!(multimap.get(&"fruits"), Some(&"apple"));
        assert_eq!(multimap.get_all(&"vegetables"), Some(&["carrot", "broccoli"][..]));

        // Pop from fruits
        assert_eq!(multimap.pop(&"fruits"), Some("cherry"));
        assert_eq!(multimap.get_all(&"fruits"), Some(&["apple", "banana"][..]));

        // Remove all vegetables
        let removed = multimap.remove_all(&"vegetables");
        assert_eq!(removed, Some(vec!["carrot", "broccoli"]));
        assert_eq!(multimap.len(), 2);

        // Modify grains
        if let Some(value) = multimap.get_mut(&"grains") {
            *value = "wheat";
        }
        assert_eq!(multimap.get(&"grains"), Some(&"wheat"));

        // Add more values
        multimap.insert("grains", "barley");
        multimap.insert("grains", "oats");

        // Test iterators
        let all_grains: Vec<_> = multimap.get_all(&"grains").unwrap().to_vec();
        assert_eq!(all_grains, vec!["wheat", "barley", "oats"]);

        // Count total values
        let total_values = multimap.values().count();
        assert_eq!(total_values, 5); // 2 fruits + 3 grains
    }

    #[test]
    fn test_edge_cases() {
        let mut multimap = MultiMap::new();

        // Empty multimap operations
        assert_eq!(multimap.keys().count(), 0);
        assert_eq!(multimap.values().count(), 0);
        assert_eq!(multimap.iter().count(), 0);

        // Single value per key
        multimap.insert(1, "one");
        assert_eq!(multimap.get(&1), Some(&"one"));
        assert_eq!(multimap.get_all(&1), Some(&["one"][..]));

        // Large number of values for single key
        let mut large_multimap = MultiMap::new();
        for i in 0..100 {
            large_multimap.insert("key", i);
        }
        assert_eq!(large_multimap.len(), 1);
        assert_eq!(large_multimap.get(&"key"), Some(&0));
        assert_eq!(large_multimap.get_all(&"key").unwrap().len(), 100);
        assert_eq!(large_multimap.pop(&"key"), Some(99));
        assert_eq!(large_multimap.get_all(&"key").unwrap().len(), 99);
    }

    #[test]
    fn test_different_types() {
        #[derive(Debug, PartialEq)]
        struct Person {
            name: String,
            age: u32,
        }

        // Test with String keys and values
        let mut string_multimap: MultiMap<String, String> = MultiMap::new();
        string_multimap.insert("key".to_string(), "value".to_string());
        assert_eq!(string_multimap.get(&"key".to_string()), Some(&"value".to_string()));

        // Test with integer keys and values
        let mut int_multimap: MultiMap<i32, i32> = MultiMap::new();
        int_multimap.insert(1, 10);
        int_multimap.insert(1, 20);
        assert_eq!(int_multimap.get_all(&1), Some(&[10, 20][..]));

        let mut person_multimap: MultiMap<&str, Person> = MultiMap::new();
        person_multimap.insert(
            "group1",
            Person {
                name: "Alice".to_string(),
                age: 30,
            },
        );
        person_multimap.insert(
            "group1",
            Person {
                name: "Bob".to_string(),
                age: 25,
            },
        );

        assert_eq!(
            person_multimap.get(&"group1"),
            Some(&Person {
                name: "Alice".to_string(),
                age: 30
            })
        );
        assert_eq!(person_multimap.get_all(&"group1").unwrap().len(), 2);
    }

    #[test]
    fn test_entry() {
        let mut multimap = MultiMap::new();

        // Test or_default on new key
        multimap.entry("key1").or_default().push(10);
        assert_eq!(multimap.get_all(&"key1"), Some(&[10][..]));

        // Test or_default on existing key
        multimap.entry("key1").or_default().push(20);
        assert_eq!(multimap.get_all(&"key1"), Some(&[10, 20][..]));

        // Test and_modify
        multimap.entry("key1").and_modify(|v| v.push(30));
        assert_eq!(multimap.get_all(&"key1"), Some(&[10, 20, 30][..]));

        // Test and_modify on non-existent key (should do nothing)
        multimap.entry("key2").and_modify(|v| v.push(100));
        assert_eq!(multimap.get_all(&"key2"), None);

        // Test or_insert
        multimap.entry("key3").or_insert(vec![1, 2, 3]);
        assert_eq!(multimap.get_all(&"key3"), Some(&[1, 2, 3][..]));

        // Test or_insert on existing key (should not replace)
        multimap.entry("key3").or_insert(vec![4, 5, 6]);
        assert_eq!(multimap.get_all(&"key3"), Some(&[1, 2, 3][..]));

        // Test or_insert_with
        multimap.entry("key4").or_insert_with(|| vec![7, 8, 9]);
        assert_eq!(multimap.get_all(&"key4"), Some(&[7, 8, 9][..]));

        // Test chaining and_modify with or_default
        multimap.entry("key5").and_modify(|v| v.push(99)).or_default().push(10);
        assert_eq!(multimap.get_all(&"key5"), Some(&[10][..]));

        // Test modifying through entry on existing key
        multimap.entry("key5").and_modify(|v| {
            v.clear();
            v.push(20);
            v.push(30);
        });
        assert_eq!(multimap.get_all(&"key5"), Some(&[20, 30][..]));
    }
}
