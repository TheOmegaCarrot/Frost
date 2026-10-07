//! [`ValueMap`], the mutable form of a Frost Map, and the types its methods return.

use std::{
    collections::{BTreeMap, btree_map},
    fmt,
    iter::FusedIterator,
    ops::Index,
    slice, vec,
};

use crate::core::{FrostFloat, MapKey, Value};

/// Most entries a Map holds in its compact form, beyond which it switches to a
/// form that stays fast as it grows.
// TODO: Rebenchmark this limit now that the compact form is kept sorted.
const COMPACT_LIMIT: usize = 16;

/// A Frost Map that you own outright and may change: the mutable form of a
/// [`FrostMap`](crate::FrostMap).
///
/// Its API follows the standard library's maps. Build one up with
/// [`insert`](Self::insert), [`entry`](Self::entry), [`Extend`], or [`FromIterator`],
/// then turn it into a `FrostMap` or [`Value`] to hand to Frost; take one back out
/// with [`FrostMap::try_into_map`](crate::FrostMap::try_into_map).
///
/// Entries iterate in ascending key order (see [`MapKey`]), so Maps that are
/// equal iterate alike, however each was built.
///
/// ```
/// use frostlang::{MapKey, Value, ValueMap};
///
/// let mut map = ValueMap::new();
/// map.insert(MapKey::from("name"), Value::from("Frost"));
/// map.insert(MapKey::from("year"), Value::Int(2026));
/// assert_eq!(map[&MapKey::from("name")], Value::from("Frost"));
///
/// let value = Value::from(map);
/// assert_eq!(value.as_map().map(|map| map.len()), Some(2));
/// ```
#[derive(Clone, Default)]
pub struct ValueMap(Repr);

#[derive(Clone)]
enum Repr {
    // Sorted by key and binary searched: for the many small Maps a script makes,
    // one allocation and a short search beat any tree or table.
    // Never holds more than `COMPACT_LIMIT` entries.
    Compact(Vec<(MapKey, Value)>),
    // Entered once the Map would hold more than `COMPACT_LIMIT` entries. Removing
    // entries does not switch back, except `clear`, which starts afresh.
    Tree(BTreeMap<MapKey, Value>),
}

impl Default for Repr {
    fn default() -> Self {
        Repr::Compact(Vec::new())
    }
}

impl ValueMap {
    /// An empty Map.
    pub fn new() -> Self {
        Self::default()
    }

    /// The number of entries.
    pub fn len(&self) -> usize {
        match &self.0 {
            Repr::Compact(entries) => entries.len(),
            Repr::Tree(tree) => tree.len(),
        }
    }

    /// Whether the Map has no entries.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The value at `key`, if present.
    pub fn get(&self, key: &MapKey) -> Option<&Value> {
        self.get_key_value(key).map(|(_, value)| value)
    }

    /// The stored key and its value for `key`, if present.
    pub fn get_key_value(&self, key: &MapKey) -> Option<(&MapKey, &Value)> {
        match &self.0 {
            Repr::Compact(entries) => {
                let (key, value) = &entries[search(entries, key).ok()?];
                Some((key, value))
            }
            Repr::Tree(tree) => tree.get_key_value(key),
        }
    }

    /// The value at String key `key`, if present.
    pub fn get_str(&self, key: &str) -> Option<&Value> {
        self.get(&MapKey::from(key))
    }

    /// The value at Bytes key `key`, if present.
    pub fn get_bytes(&self, key: &[u8]) -> Option<&Value> {
        self.get(&MapKey::from(key))
    }

    /// The value at Int key `key`, if present.
    pub fn get_int(&self, key: i64) -> Option<&Value> {
        self.get(&MapKey::Int(key))
    }

    /// The value at Bool key `key`, if present.
    pub fn get_bool(&self, key: bool) -> Option<&Value> {
        self.get(&MapKey::Bool(key))
    }

    /// The value at Float key `key`, if present.
    pub fn get_float(&self, key: FrostFloat) -> Option<&Value> {
        self.get(&MapKey::Float(key))
    }

    /// A mutable reference to the value at `key`, if present.
    pub fn get_mut(&mut self, key: &MapKey) -> Option<&mut Value> {
        match &mut self.0 {
            Repr::Compact(entries) => {
                let index = search(entries, key).ok()?;
                Some(&mut entries[index].1)
            }
            Repr::Tree(tree) => tree.get_mut(key),
        }
    }

    /// Whether the Map holds `key`.
    pub fn contains_key(&self, key: &MapKey) -> bool {
        self.get(key).is_some()
    }

    /// Set `key` to `value`, returning the value it replaced, if any.
    ///
    /// A key already present is not replaced: the Map keeps the key it stored
    /// first. This matters only for keys that are equal but distinguishable, such
    /// as Float `0.0` and `-0.0`.
    pub fn insert(&mut self, key: MapKey, value: Value) -> Option<Value> {
        match self.entry(key) {
            Entry::Occupied(mut entry) => Some(entry.insert(value)),
            Entry::Vacant(entry) => {
                entry.insert(value);
                None
            }
        }
    }

    /// Remove `key`, returning its value, if it was present.
    pub fn remove(&mut self, key: &MapKey) -> Option<Value> {
        self.remove_entry(key).map(|(_, value)| value)
    }

    /// Remove `key`, returning the stored key and its value, if it was present.
    pub fn remove_entry(&mut self, key: &MapKey) -> Option<(MapKey, Value)> {
        match &mut self.0 {
            Repr::Compact(entries) => {
                let index = search(entries, key).ok()?;
                Some(entries.remove(index))
            }
            Repr::Tree(tree) => tree.remove_entry(key),
        }
    }

    /// Keep only the entries for which `keep` returns true.
    pub fn retain(&mut self, mut keep: impl FnMut(&MapKey, &mut Value) -> bool) {
        match &mut self.0 {
            Repr::Compact(entries) => entries.retain_mut(|(key, value)| keep(key, value)),
            Repr::Tree(tree) => tree.retain(|key, value| keep(key, value)),
        }
    }

    /// Remove every entry.
    pub fn clear(&mut self) {
        match &mut self.0 {
            Repr::Compact(entries) => entries.clear(),
            Repr::Tree(_) => self.0 = Repr::default(),
        }
    }

    /// The entry for `key`, for in-place inspection or insertion.
    pub fn entry(&mut self, key: MapKey) -> Entry<'_> {
        // A full compact Map switches form before a new key can go in, so a vacant
        // compact entry always has room.
        if let Repr::Compact(entries) = &mut self.0
            && entries.len() == COMPACT_LIMIT
            && search(entries, &key).is_err()
        {
            self.0 = Repr::Tree(std::mem::take(entries).into_iter().collect());
        }
        match &mut self.0 {
            Repr::Compact(entries) => match search(entries, &key) {
                Ok(index) => {
                    Entry::Occupied(OccupiedEntry(OccupiedRepr::Compact { entries, index }))
                }
                Err(index) => Entry::Vacant(VacantEntry(VacantRepr::Compact {
                    entries,
                    index,
                    key,
                })),
            },
            Repr::Tree(tree) => match tree.entry(key) {
                btree_map::Entry::Occupied(entry) => {
                    Entry::Occupied(OccupiedEntry(OccupiedRepr::Tree(entry)))
                }
                btree_map::Entry::Vacant(entry) => {
                    Entry::Vacant(VacantEntry(VacantRepr::Tree(entry)))
                }
            },
        }
    }

    /// The entries.
    pub fn iter(&self) -> Iter<'_> {
        Iter(match &self.0 {
            Repr::Compact(entries) => IterRepr::Compact(entries.iter()),
            Repr::Tree(tree) => IterRepr::Tree(tree.iter()),
        })
    }

    /// The entries, with mutable values.
    pub fn iter_mut(&mut self) -> IterMut<'_> {
        IterMut(match &mut self.0 {
            Repr::Compact(entries) => IterMutRepr::Compact(entries.iter_mut()),
            Repr::Tree(tree) => IterMutRepr::Tree(tree.iter_mut()),
        })
    }

    /// The keys.
    pub fn keys(&self) -> Keys<'_> {
        Keys(self.iter())
    }

    /// The values.
    pub fn values(&self) -> Values<'_> {
        Values(self.iter())
    }

    /// The values, mutably.
    pub fn values_mut(&mut self) -> ValuesMut<'_> {
        ValuesMut(self.iter_mut())
    }

    /// The keys, consuming the Map.
    pub fn into_keys(self) -> IntoKeys {
        IntoKeys(self.into_iter())
    }

    /// The values, consuming the Map.
    pub fn into_values(self) -> IntoValues {
        IntoValues(self.into_iter())
    }
}

/// Where `key` is in the compact form's `entries`: `Ok` with its index if
/// present, else `Err` with the index that keeps the entries sorted if it were
/// inserted there.
fn search(entries: &[(MapKey, Value)], key: &MapKey) -> Result<usize, usize> {
    entries.binary_search_by(|(candidate, _)| candidate.cmp(key))
}

impl PartialEq for ValueMap {
    fn eq(&self, other: &Self) -> bool {
        // Both forms iterate in key order, so equal Maps iterate alike.
        self.len() == other.len() && self.iter().eq(other.iter())
    }
}

impl Eq for ValueMap {}

impl fmt::Debug for ValueMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl Index<&MapKey> for ValueMap {
    type Output = Value;

    /// The value at `key`.
    ///
    /// # Panics
    ///
    /// If the Map does not hold `key`.
    fn index(&self, key: &MapKey) -> &Value {
        self.get(key).expect("no entry found for key")
    }
}

impl Extend<(MapKey, Value)> for ValueMap {
    /// [`Insert`](ValueMap::insert) each entry in turn.
    fn extend<T: IntoIterator<Item = (MapKey, Value)>>(&mut self, entries: T) {
        for (key, value) in entries {
            self.insert(key, value);
        }
    }
}

impl FromIterator<(MapKey, Value)> for ValueMap {
    /// A Map of the entries, each [inserted](ValueMap::insert) in turn.
    fn from_iter<T: IntoIterator<Item = (MapKey, Value)>>(entries: T) -> Self {
        let entries = entries.into_iter();
        // Room for as many entries as the compact form holds, if the iterator may
        // yield that many. The upper bound serves fallible collections, whose lower
        // bound is zero; the cap bounds what an overestimate wastes.
        let (lower, upper) = entries.size_hint();
        let room = upper.unwrap_or(lower).min(COMPACT_LIMIT);
        let mut map = Self(Repr::Compact(Vec::with_capacity(room)));
        map.extend(entries);
        map
    }
}

impl<const N: usize> From<[(MapKey, Value); N]> for ValueMap {
    /// A Map of the entries, each [inserted](ValueMap::insert) in turn.
    fn from(entries: [(MapKey, Value); N]) -> Self {
        entries.into_iter().collect()
    }
}

impl From<BTreeMap<MapKey, Value>> for ValueMap {
    fn from(tree: BTreeMap<MapKey, Value>) -> Self {
        if tree.len() > COMPACT_LIMIT {
            Self(Repr::Tree(tree))
        } else {
            Self(Repr::Compact(tree.into_iter().collect()))
        }
    }
}

// -- Entries --

/// A view into one entry of a [`ValueMap`], from [`ValueMap::entry`]: either
/// occupied or vacant.
#[derive(Debug)]
pub enum Entry<'a> {
    /// The Map holds the key.
    Occupied(OccupiedEntry<'a>),
    /// The Map does not hold the key.
    Vacant(VacantEntry<'a>),
}

impl<'a> Entry<'a> {
    /// The entry's key.
    pub fn key(&self) -> &MapKey {
        match self {
            Entry::Occupied(entry) => entry.key(),
            Entry::Vacant(entry) => entry.key(),
        }
    }

    /// The entry's value, inserting `default` first if the entry is vacant.
    pub fn or_insert(self, default: Value) -> &'a mut Value {
        match self {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(default),
        }
    }

    /// The entry's value, inserting the result of `default` first if the entry is
    /// vacant.
    pub fn or_insert_with(self, default: impl FnOnce() -> Value) -> &'a mut Value {
        match self {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(default()),
        }
    }

    /// The entry's value, inserting the result of `default`, given the entry's key,
    /// first if the entry is vacant.
    pub fn or_insert_with_key(self, default: impl FnOnce(&MapKey) -> Value) -> &'a mut Value {
        match self {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let value = default(entry.key());
                entry.insert(value)
            }
        }
    }

    /// The entry's value, inserting [`Value::Null`] first if the entry is vacant.
    pub fn or_default(self) -> &'a mut Value {
        self.or_insert(Value::Null)
    }

    /// Apply `modify` to the value if the entry is occupied; either way, return
    /// the entry.
    pub fn and_modify(mut self, modify: impl FnOnce(&mut Value)) -> Self {
        if let Entry::Occupied(entry) = &mut self {
            modify(entry.get_mut());
        }
        self
    }
}

/// An occupied entry of a [`ValueMap`]: part of an [`Entry`].
pub struct OccupiedEntry<'a>(OccupiedRepr<'a>);

enum OccupiedRepr<'a> {
    Compact {
        entries: &'a mut Vec<(MapKey, Value)>,
        index: usize,
    },
    Tree(btree_map::OccupiedEntry<'a, MapKey, Value>),
}

impl<'a> OccupiedEntry<'a> {
    /// The entry's key.
    pub fn key(&self) -> &MapKey {
        match &self.0 {
            OccupiedRepr::Compact { entries, index } => &entries[*index].0,
            OccupiedRepr::Tree(entry) => entry.key(),
        }
    }

    /// The entry's value.
    pub fn get(&self) -> &Value {
        match &self.0 {
            OccupiedRepr::Compact { entries, index } => &entries[*index].1,
            OccupiedRepr::Tree(entry) => entry.get(),
        }
    }

    /// The entry's value, mutably.
    pub fn get_mut(&mut self) -> &mut Value {
        match &mut self.0 {
            OccupiedRepr::Compact { entries, index } => &mut entries[*index].1,
            OccupiedRepr::Tree(entry) => entry.get_mut(),
        }
    }

    /// The entry's value, mutably, borrowing the Map rather than the entry.
    pub fn into_mut(self) -> &'a mut Value {
        match self.0 {
            OccupiedRepr::Compact { entries, index } => &mut entries[index].1,
            OccupiedRepr::Tree(entry) => entry.into_mut(),
        }
    }

    /// Replace the entry's value, returning the old one.
    pub fn insert(&mut self, value: Value) -> Value {
        std::mem::replace(self.get_mut(), value)
    }

    /// Remove the entry, returning its value.
    pub fn remove(self) -> Value {
        self.remove_entry().1
    }

    /// Remove the entry, returning its key and value.
    pub fn remove_entry(self) -> (MapKey, Value) {
        match self.0 {
            OccupiedRepr::Compact { entries, index } => entries.remove(index),
            OccupiedRepr::Tree(entry) => entry.remove_entry(),
        }
    }
}

impl fmt::Debug for OccupiedEntry<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OccupiedEntry")
            .field("key", self.key())
            .field("value", self.get())
            .finish()
    }
}

/// A vacant entry of a [`ValueMap`]: part of an [`Entry`].
pub struct VacantEntry<'a>(VacantRepr<'a>);

enum VacantRepr<'a> {
    Compact {
        entries: &'a mut Vec<(MapKey, Value)>,
        // Where `key` goes to keep the entries sorted.
        index: usize,
        key: MapKey,
    },
    Tree(btree_map::VacantEntry<'a, MapKey, Value>),
}

impl<'a> VacantEntry<'a> {
    /// The key the entry would be inserted under.
    pub fn key(&self) -> &MapKey {
        match &self.0 {
            VacantRepr::Compact { key, .. } => key,
            VacantRepr::Tree(entry) => entry.key(),
        }
    }

    /// Take back the key, inserting nothing.
    pub fn into_key(self) -> MapKey {
        match self.0 {
            VacantRepr::Compact { key, .. } => key,
            VacantRepr::Tree(entry) => entry.into_key(),
        }
    }

    /// Insert `value` under the entry's key, returning it mutably.
    pub fn insert(self, value: Value) -> &'a mut Value {
        match self.0 {
            VacantRepr::Compact {
                entries,
                index,
                key,
            } => {
                entries.insert(index, (key, value));
                &mut entries[index].1
            }
            VacantRepr::Tree(entry) => entry.insert(value),
        }
    }
}

impl fmt::Debug for VacantEntry<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("VacantEntry").field(self.key()).finish()
    }
}

// -- Iterators --

/// An iterator over a [`ValueMap`]'s entries, from [`ValueMap::iter`].
#[derive(Clone)]
pub struct Iter<'a>(IterRepr<'a>);

#[derive(Clone)]
enum IterRepr<'a> {
    Compact(slice::Iter<'a, (MapKey, Value)>),
    Tree(btree_map::Iter<'a, MapKey, Value>),
}

impl<'a> Iterator for Iter<'a> {
    type Item = (&'a MapKey, &'a Value);

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            IterRepr::Compact(entries) => entries.next().map(|(key, value)| (key, value)),
            IterRepr::Tree(tree) => tree.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.0 {
            IterRepr::Compact(entries) => entries.size_hint(),
            IterRepr::Tree(tree) => tree.size_hint(),
        }
    }
}

impl ExactSizeIterator for Iter<'_> {}

impl FusedIterator for Iter<'_> {}

impl fmt::Debug for Iter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

impl<'a> IntoIterator for &'a ValueMap {
    type Item = (&'a MapKey, &'a Value);
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// An iterator over a [`ValueMap`]'s entries with mutable values, from
/// [`ValueMap::iter_mut`].
pub struct IterMut<'a>(IterMutRepr<'a>);

enum IterMutRepr<'a> {
    Compact(slice::IterMut<'a, (MapKey, Value)>),
    Tree(btree_map::IterMut<'a, MapKey, Value>),
}

impl<'a> Iterator for IterMut<'a> {
    type Item = (&'a MapKey, &'a mut Value);

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            IterMutRepr::Compact(entries) => entries.next().map(|(key, value)| (&*key, value)),
            IterMutRepr::Tree(tree) => tree.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.0 {
            IterMutRepr::Compact(entries) => entries.size_hint(),
            IterMutRepr::Tree(tree) => tree.size_hint(),
        }
    }
}

impl ExactSizeIterator for IterMut<'_> {}

impl FusedIterator for IterMut<'_> {}

impl fmt::Debug for IterMut<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IterMut").finish_non_exhaustive()
    }
}

impl<'a> IntoIterator for &'a mut ValueMap {
    type Item = (&'a MapKey, &'a mut Value);
    type IntoIter = IterMut<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

/// An owning iterator over a [`ValueMap`]'s entries.
pub struct IntoIter(IntoIterRepr);

enum IntoIterRepr {
    Compact(vec::IntoIter<(MapKey, Value)>),
    Tree(btree_map::IntoIter<MapKey, Value>),
}

impl Iterator for IntoIter {
    type Item = (MapKey, Value);

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            IntoIterRepr::Compact(entries) => entries.next(),
            IntoIterRepr::Tree(tree) => tree.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.0 {
            IntoIterRepr::Compact(entries) => entries.size_hint(),
            IntoIterRepr::Tree(tree) => tree.size_hint(),
        }
    }
}

impl ExactSizeIterator for IntoIter {}

impl FusedIterator for IntoIter {}

impl fmt::Debug for IntoIter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IntoIter").finish_non_exhaustive()
    }
}

impl IntoIterator for ValueMap {
    type Item = (MapKey, Value);
    type IntoIter = IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(match self.0 {
            Repr::Compact(entries) => IntoIterRepr::Compact(entries.into_iter()),
            Repr::Tree(tree) => IntoIterRepr::Tree(tree.into_iter()),
        })
    }
}

// Iterator, ExactSizeIterator, and FusedIterator for a wrapper that projects each
// item of the entry iterator it holds.
macro_rules! projecting_iterator {
    ($name:ident $(<$lifetime:lifetime>)?, $item:ty, |$entry:pat_param| $projection:expr) => {
        impl<$($lifetime)?> Iterator for $name<$($lifetime)?> {
            type Item = $item;

            fn next(&mut self) -> Option<Self::Item> {
                self.0.next().map(|$entry| $projection)
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                self.0.size_hint()
            }
        }

        impl<$($lifetime)?> ExactSizeIterator for $name<$($lifetime)?> {}

        impl<$($lifetime)?> FusedIterator for $name<$($lifetime)?> {}
    };
}

/// An iterator over a [`ValueMap`]'s keys, from [`ValueMap::keys`].
#[derive(Clone)]
pub struct Keys<'a>(Iter<'a>);

projecting_iterator!(Keys<'a>, &'a MapKey, |(key, _)| key);

impl fmt::Debug for Keys<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

/// An iterator over a [`ValueMap`]'s values, from [`ValueMap::values`].
#[derive(Clone)]
pub struct Values<'a>(Iter<'a>);

projecting_iterator!(Values<'a>, &'a Value, |(_, value)| value);

impl fmt::Debug for Values<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

/// An iterator over a [`ValueMap`]'s values, mutably, from [`ValueMap::values_mut`].
pub struct ValuesMut<'a>(IterMut<'a>);

projecting_iterator!(ValuesMut<'a>, &'a mut Value, |(_, value)| value);

impl fmt::Debug for ValuesMut<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ValuesMut").finish_non_exhaustive()
    }
}

/// An owning iterator over a [`ValueMap`]'s keys, from [`ValueMap::into_keys`].
pub struct IntoKeys(IntoIter);

projecting_iterator!(IntoKeys, MapKey, |(key, _)| key);

impl fmt::Debug for IntoKeys {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IntoKeys").finish_non_exhaustive()
    }
}

/// An owning iterator over a [`ValueMap`]'s values, from [`ValueMap::into_values`].
pub struct IntoValues(IntoIter);

projecting_iterator!(IntoValues, Value, |(_, value)| value);

impl fmt::Debug for IntoValues {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IntoValues").finish_non_exhaustive()
    }
}
