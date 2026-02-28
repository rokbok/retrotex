use std::hash::{DefaultHasher, Hash, Hasher};

pub fn quick_hash<T: Hash>(t: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    t.hash(&mut hasher);
    hasher.finish()
}
