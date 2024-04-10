use crate::bpe::{Unit, UnitMapInternal};
use std::collections::HashMap;
use std::fmt;

mod config;

pub use config::DictionaryConfig;

// TODO: serde file
pub struct Dictionary {
    words: HashMap<Vec<u8>, usize>,  // <words, appearance>
}

impl Dictionary {
    pub fn empty() -> Self {
        Dictionary { words: HashMap::new() }
    }

    pub fn from_units(units: &[Unit], unit_map: &UnitMapInternal) -> Self {
        let mut words = HashMap::with_capacity(unit_map.len());

        for unit in units.iter() {
            let word = unit_map.get(unit).unwrap().to_vec();

            match words.get_mut(&word) {
                Some(n) => {
                    *n += 1;
                },
                None => {
                    words.insert(word, 1);
                },
            }
        }

        // if `keep_single_byte_tokens` is on, there can be tokens without any appearance
        for byte in 0..256 {
            if unit_map.contains_key(&byte) && !words.contains_key(&vec![byte as u8]) {
                words.insert(vec![byte as u8], 0);
            }
        }

        Dictionary { words }
    }

    pub fn get_words_as_strings(&self) -> Vec<String> {
        self.words.keys().map(
            |word| String::from_utf8_lossy(word).to_string()
        ).collect()
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<Vec<u8>, usize> {
        self.words.iter()
    }

    pub fn get<Q>(&self, word: &Q) -> Option<usize>
    where Vec<u8>: std::borrow::Borrow<Q>, Q: Eq + std::hash::Hash {
        self.words.get(word).map(|appearance| *appearance)
    }

    pub fn len(&self) -> usize {
        self.words.len()
    }

    pub fn merge(&mut self, other: &Dictionary) {
        for (word, appearance) in other.iter() {
            match self.words.get_mut(word) {
                Some(n) => {
                    *n += *appearance;
                },
                None => {
                    self.words.insert(word.to_vec(), *appearance);
                },
            }
        }
    }

    /// a `u32` value represents a token\
    pub fn tokenize(&self, s: &[u8]) -> (Vec<u32>, HashMap<u32, Vec<u8>>) {
        let mut ordered_words = self.words.iter().collect::<Vec<_>>();
        ordered_words.sort_by_key(|(_, appearance)| usize::MAX - *appearance);

        // TODO: below is just a pseudo-code
        // for (word, _) in ordered_words.iter() {
        //     for c in s.chunks() {
        //         if c == word {
        //             replace(c, word)
        //         }
        //     }
        // }

        todo!()
    }
}

impl fmt::Debug for Dictionary {
    /// It takes long time because it sorts the words by appearance.
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let mut v = self.words.iter().map(
            |(word, appearance)| (
                String::from_utf8_lossy(word).to_string(),
                *appearance,
            )
        ).filter(
            |(_, appearance)| *appearance > 0
        ).collect::<Vec<_>>();

        v.sort_by_key(|(_, appearance)| usize::MAX - appearance);

        write!(fmt, "{v:?}")
    }
}
