use std::any::Any;
use std::fmt::{self, Debug};
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};
use std::sync::atomic::Ordering;

use portable_atomic::AtomicU128;
use serde::{Serialize, Serializer};
use siphasher::sip128::{Hasher128, SipHasher13};
use crate::tick;

/// A wrapper type with lazily-computed hash.
///
/// This is useful if you want to pass large values of `T` to memoized
/// functions. Especially recursive structures like trees benefit from
/// intermediate prehashed nodes.
///
/// Note that for a value `v` of type `T`, `hash(v)` is not necessarily equal to
/// `hash(LazyHash::new(v))`. Writing the precomputed hash into a hasher's
/// state produces different output than writing the value's parts directly.
/// However, that seldom matters as you are typically either dealing with values
/// of type `T` or with values of type `LazyHash<T>`, not a mix of both.
///
/// # Equality
/// Because Typst uses high-quality 128 bit hashes in all places, the risk of a
/// hash collision is reduced to an absolute minimum. Therefore, this type
/// additionally provides `PartialEq` and `Eq` implementations that compare by
/// hash instead of by value. For this to be correct, your hash implementation
/// **must feed all information relevant to the `PartialEq` impl to the
/// hasher.**
///
/// # Usage
/// If the value is expected to be cloned, it is best used inside of an `Arc`
/// or `Rc` to best re-use the hash once it has been computed.
pub struct LazyHash<T: ?Sized> {
    /// The hash for the value.
    hash: AtomicU128,
    /// The underlying value.
    value: T,
}

impl<T: Default> Default for LazyHash<T> {
    #[inline]
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<T> LazyHash<T> {
    /// Wraps an item without pre-computed hash.
    #[inline]
    pub fn new(value: T) -> Self {
        Self { hash: AtomicU128::new(0), value }
    }

    /// Wrap an item with a pre-computed hash.
    ///
    /// **Important:** The hash must be correct for the value. This cannot be
    /// enforced at compile time, so use with caution.
    #[inline]
    pub fn reuse<U: ?Sized>(value: T, existing: &LazyHash<U>) -> Self {
        LazyHash { hash: AtomicU128::new(existing.load_hash()), value }
    }

    /// Returns the wrapped value.
    #[inline]
    pub fn into_inner(self) -> T {
        self.value
    }
}

impl<T: ?Sized> LazyHash<T> {
    /// Get the hash, returns zero if not computed yet.
    #[inline]
    fn load_hash(&self) -> u128 {
        // We only need atomicity and no synchronization of other operations, so
        // `Relaxed` is fine.
        self.hash.load(Ordering::Relaxed)
    }
}

impl<T: Hash + ?Sized + 'static> LazyHash<T> {
    /// Get the hash or compute it if not set yet.
    #[inline]
    fn load_or_compute_hash(&self) -> u128 {
        let mut hash = self.load_hash();
        if hash == 0 {
            hash = hash_item(&self.value);
            self.hash.store(hash, Ordering::Relaxed);
        }
        hash
    }

    /// Reset the hash to zero.
    #[inline]
    fn reset_hash(&mut self) {
        // Because we have a mutable reference, we can skip the atomic.
        *self.hash.get_mut() = 0;
    }
}

/// Hash the item.
#[inline]
fn hash_item<T: Hash + ?Sized + 'static>(item: &T) -> u128 {
    // Also hash the TypeId because the type might be converted
    // through an unsized coercion.
    let mut state = SipHasher13::new();
    item.type_id().hash(&mut state);
    item.hash(&mut state);
    state.finish128().as_u128()
}

impl<T: Hash + ?Sized + 'static> Hash for LazyHash<T> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u128(self.load_or_compute_hash());
    }
}

impl<T> From<T> for LazyHash<T> {
    #[inline]
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T: Hash + ?Sized + 'static> Eq for LazyHash<T> {}

impl<T: Hash + ?Sized + 'static> PartialEq for LazyHash<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.load_or_compute_hash() == other.load_or_compute_hash()
    }
}

impl<T: ?Sized> Deref for LazyHash<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T: Hash + ?Sized + 'static> DerefMut for LazyHash<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.reset_hash();
        &mut self.value
    }
}

impl<T: Hash + Clone + 'static> Clone for LazyHash<T> {
    fn clone(&self) -> Self {
        Self {
            hash: AtomicU128::new(self.load_hash()),
            value: self.value.clone(),
        }
    }
}

impl<T: Debug> Debug for LazyHash<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.value.fmt(f)
    }
}

/// A wrapper type with a manually computed hash.
///
/// This can be used to turn an unhashable type into a hashable one where the
/// hash is provided manually. Typically, the hash is derived from the data
/// which was used to construct to the unhashable type.
///
/// For instance, you could hash the bytes that were parsed into an unhashable
/// data structure.
///
/// # Equality
/// Because Typst uses high-quality 128 bit hashes in all places, the risk of a
/// hash collision is reduced to an absolute minimum. Therefore, this type
/// additionally provides `PartialEq` and `Eq` implementations that compare by
/// hash instead of by value. For this to be correct, your hash implementation
/// **must feed all information relevant to the `PartialEq` impl to the
/// hasher.**
#[derive(Clone)]
pub struct ManuallyHash<T: ?Sized> {
    /// A manually computed hash.
    hash: u128,
    /// The underlying value.
    value: T,
}

impl<T> ManuallyHash<T> {
    /// Wraps an item with a pre-computed hash.
    ///
    /// The hash should be computed with `typst_utils::hash128`.
    #[inline]
    pub fn new(value: T, hash: u128) -> Self {
        Self { hash, value }
    }

    /// Returns the wrapped value.
    #[inline]
    pub fn into_inner(self) -> T {
        self.value
    }
}

impl<T: ?Sized> Hash for ManuallyHash<T> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u128(self.hash);
    }
}

impl<T: ?Sized> Eq for ManuallyHash<T> {}

impl<T: ?Sized> PartialEq for ManuallyHash<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

impl<T: ?Sized> Deref for ManuallyHash<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T: Debug> Debug for ManuallyHash<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.value.fmt(f)
    }
}

// TODO
impl <T : Serialize + Debug> Serialize for LazyHash<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        tick!("{:?}", self);
        let result = self.value.serialize(serializer);
        tick!();
        result
    }
}

