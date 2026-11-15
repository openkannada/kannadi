//! Longest-prefix lookup over string keys, shared by the transliteration schemes and
//! the legacy glyph table.

use std::collections::HashMap;

pub(crate) struct Lookup<T> {
    entries: HashMap<String, T>,
    longest: usize,
}

impl<T> Lookup<T> {
    pub(crate) fn new() -> Self {
        Lookup {
            entries: HashMap::new(),
            longest: 0,
        }
    }

    pub(crate) fn insert(&mut self, key: impl Into<String>, value: T) {
        let key = key.into();
        self.longest = self.longest.max(key.chars().count());
        self.entries.insert(key, value);
    }

    pub(crate) fn get(&self, key: &str) -> Option<&T> {
        self.entries.get(key)
    }

    pub(crate) fn keys(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }

    /// The value of the longest key that starts `input`, and that key's length in chars.
    pub(crate) fn longest_match(
        &self,
        input: impl IntoIterator<Item = char>,
    ) -> Option<(&T, usize)> {
        let mut key = String::new();
        let mut best = None;
        for (n, c) in input.into_iter().take(self.longest).enumerate() {
            key.push(c);
            if let Some(value) = self.entries.get(&key) {
                best = Some((value, n + 1));
            }
        }
        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn longest_key_wins() {
        let mut l = Lookup::new();
        l.insert("k", 1);
        l.insert("kh", 2);
        l.insert("khx", 3);
        assert_eq!(l.longest_match("khaa".chars()), Some((&2, 2)));
        assert_eq!(l.longest_match("k".chars()), Some((&1, 1)));
        assert_eq!(l.longest_match("a".chars()), None);
        assert_eq!(l.longest_match("".chars()), None);
    }
}
