use std::collections::HashMap;

pub fn dedupe_by_key<T, K, F>(items: &[T], key_fn: F) -> Vec<T>
where
    T: Clone,
    K: std::hash::Hash + Eq,
    F: Fn(&T) -> K,
{
    let mut seen = HashMap::new();
    for item in items {
        seen.entry(key_fn(item)).or_insert_with(|| item.clone());
    }
    seen.into_values().collect()
}
