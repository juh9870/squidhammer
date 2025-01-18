pub type OrderMapEntry<'a, K, V> = ordermap::map::Entry<'a, K, V>;

#[allow(clippy::disallowed_types)]
pub type Hasher = ahash::AHasher;
pub type BuildHasher = std::hash::BuildHasherDefault<Hasher>;

// DOS is of no concern to us
pub type OrderMap<K, V> = ordermap::OrderMap<K, V, BuildHasher>;
pub type OrderSet<V> = ordermap::OrderSet<V, BuildHasher>;

#[allow(clippy::disallowed_types)]
pub type HashMap<K, V> = std::collections::HashMap<K, V, BuildHasher>;
#[allow(clippy::disallowed_types)]
pub type HashSet<V> = std::collections::HashSet<V, BuildHasher>;

pub fn hash_of<T: std::hash::Hash>(t: &T) -> u64 {
    let mut h = Hasher::default();
    t.hash(&mut h);
    std::hash::Hasher::finish(&h)
}
