use crate::dictionary::{Dictionary, DictionaryConfig};
use crate::files::{FileError, extension, file_size, merge_files, read_dir};
use smallvec::{SmallVec, smallvec};
use std::collections::{HashMap, HashSet};

#[cfg(test)]
mod tests;

/// It stops iteration if the string gets too small
pub const MINIMUN_STRING_LENGTH: usize = 16;

// 0 ~ 255: byte
// 256 ~ : token
pub type Unit = u32;

// two `Unit`s (naively concatonated)
// later it's assigned to a single unit (with new value)
pub type Pair = u64;

// It shall include all the single-byte units
// for performance reasons, the internal type uses `SmallVec` instead of `Vec`
pub type UnitMapInternal = HashMap<Unit, SmallVec<[u8; 4]>>;

// only including single-byte units
pub fn default_unit_map() -> UnitMapInternal {
    let mut result = HashMap::with_capacity(256);

    for c in 0..=255 {
        result.insert(c as Unit, smallvec![c]);
    }

    result
}

pub fn into_pair(c1: Unit, c2: Unit) -> Pair {
    ((c1 as Pair) << 32) | c2 as Pair
}

pub fn from_pair(p: Pair) -> (Unit, Unit) {
    ((p >> 32) as Unit, (p & 0xffff_ffff) as Unit)
}

pub fn bytes_to_units(bytes: &[u8]) -> Vec<Unit> {
    bytes.iter().map(|byte| *byte as Unit).collect()
}

// for now, it's only used for tests
#[cfg(test)]
pub fn units_to_bytes(
    units: &[Unit],

    // It includes single-byte units
    unit_map: &UnitMapInternal,
) -> Vec<u8> {
    let mut result = Vec::with_capacity(units.len() * 3);

    for c in units.iter() {
        // it's the caller's responsibility to guarantee that the map is valid
        let bytes = unit_map.get(c).unwrap();

        for byte in bytes.iter() {
            result.push(*byte);
        }
    }

    result
}

/// single-threaded version
pub fn construct_dictionary_from_dir(
    config: DictionaryConfig,
) -> Result<Dictionary, FileError> {
    let files = read_dir(&config.dir_option.path)?;
    let ext_to_see = Some(config.dir_option.ext.clone());

    let mut files_with_sizes = files.iter().filter(
        |file| match (extension(file), file_size(file)) {
            (Ok(ext), Ok(size)) if ext == ext_to_see => true,
            _ => false,
        }
    ).map(
        |file| (file.to_string(), file_size(file).unwrap())
    ).collect::<Vec<_>>();

    files_with_sizes.sort_by_key(|(_, size)| *size);

    let mut dictionary = Dictionary::empty();

    loop {
        let mut files_to_read = vec![];
        let mut curr_chunk_size = 0;

        // TODO: divide big files
        while curr_chunk_size < config.dir_option.file_chunk_size && !files_with_sizes.is_empty() {
            files_to_read.push(files_with_sizes[0].0.clone());
            curr_chunk_size += files_with_sizes[0].1 as usize;

            // It's O(n^2), but wouldn't be a bottleneck
            files_with_sizes = files_with_sizes[1..].to_vec();
        }

        // TODO: make it parallel
        // TODO: 그냥 parallel version만 남기고 이건 버리셈
        // It's very easy to make it parallel: send `files_to_read` to each thread and receive `new_dictionary`
        // from each thread
        let bytes = merge_files(files_to_read, config.dir_option.file_separator);
        let new_dictionary = construct_dictionary(&bytes, config.clone());

        dictionary.merge(&new_dictionary);

        if files_with_sizes.is_empty() {
            break;
        }
    }

    Ok(dictionary)
}

pub fn construct_dictionary(
    bytes: &[u8],
    config: DictionaryConfig,
) -> Dictionary {
    let mut unit_map = default_unit_map();
    let mut units = bytes_to_units(bytes);

    loop {
        let (_units, nothing_to_compress) = step(&units, &mut unit_map, config.minimum_appearance.unwrap_or(2), config.ultimate_separator);
        units = _units;

        if nothing_to_compress || units.len() <= MINIMUN_STRING_LENGTH {
            remove_unnecessary_units_in_map(&units, &mut unit_map, config.keep_single_byte_tokens);
            break;
        }

        if unit_map.len() >= config.dictionary_size {
            remove_unnecessary_units_in_map(&units, &mut unit_map, config.keep_single_byte_tokens);

            if unit_map.len() >= config.dictionary_size {
                break;
            }
        }
    }

    Dictionary::from_units(&units, &unit_map)
}

/// count_pairs + assign_pair_to_new_unit\
/// It also inserts an entry to `unit_map`
pub fn step(
    s: &[Unit],
    unit_map: &mut UnitMapInternal,
    minimum_appearance: usize,
    ultimate_separator: Option<u8>,
) -> (Vec<Unit>, bool) {  // (new_s, less than minimum_appearance)
    let pairs = count_pairs(s);

    let mut curr_best_pair = 0;
    let mut curr_best_count = 0;

    for (pair, count) in pairs.iter() {
        if *count > curr_best_count {
            if let Some(u) = ultimate_separator {
                let u = u as Unit;
                let (c1, c2) = from_pair(*pair);

                if u == c1 || u == c2 {
                    continue;
                }
            }

            curr_best_count = *count;
            curr_best_pair = *pair;
        }
    }

    if curr_best_count < minimum_appearance {
        return (s.to_vec(), true);
    }

    let new_unit = assign_new_unit(curr_best_pair, unit_map, None);

    (assign_pair_to_new_unit(s, curr_best_pair, new_unit), false)
}

pub fn remove_unnecessary_units_in_map(
    units: &[Unit],
    unit_map: &mut UnitMapInternal,
    keep_single_byte_tokens: bool,
) -> usize {  // it returns how many units it removed
    let mut unit_set = HashSet::with_capacity(unit_map.len());

    for c in units.iter() {
        unit_set.insert(*c);
    }

    let mut units_to_remove = vec![];

    for unit in unit_map.keys() {
        if !unit_set.contains(unit) {
            if keep_single_byte_tokens && *unit < 256 {
                continue;
            }

            units_to_remove.push(*unit);
        }
    }

    for unit in units_to_remove.iter() {
        unit_map.remove(unit);
    }

    units_to_remove.len()
}

/// It's your responsibility to guarantee that the unit_map is valid.
pub fn assign_new_unit(
    pair: Pair,
    unit_map: &mut UnitMapInternal,

    // if you have computed this in advance, you can directly provide this
    // otherwise it would calculate which unit to use.
    // if the provided unit is already being used, it would choose another one
    new_unit: Option<Unit>,
) -> Unit {
    let new_unit = match new_unit {
        Some(new_unit) if !unit_map.contains_key(&new_unit) => new_unit,
        _ => {
            let mut new_unit = 0;

            // it makes sure that multi-byte tokens are always assigned to < 255
            for i in 256..Unit::MAX {
                if !unit_map.contains_key(&i) {
                    new_unit = i;
                    break;
                }
            }

            new_unit
        },
    };

    let (c1, c2) = from_pair(pair);
    let new_bytes = vec![
        unit_map.get(&c1).unwrap().to_vec(),
        unit_map.get(&c2).unwrap().to_vec(),
    ].concat();

    unit_map.insert(new_unit, new_bytes.into());
    new_unit
}

pub fn count_pairs(s: &[Unit]) -> HashMap<Pair, usize> {
    let mut result = HashMap::with_capacity(1024);

    for p in s.windows(2) {
        let curr_pair = into_pair(p[0], p[1]);

        match result.get_mut(&curr_pair) {
            Some(n) => {
                *n += 1;
            },
            None => {
                result.insert(curr_pair, 1);
            },
        }
    }

    result
}

pub fn assign_pair_to_new_unit(s: &[Unit], pair: Pair, new_unit: Unit) -> Vec<Unit> {
    let mut result = Vec::with_capacity(s.len());
    let (c1, c2) = from_pair(pair);
    let mut expecting_c2 = false;

    for c in s.iter() {
        if expecting_c2 {
            if *c == c2 {
                result.push(new_unit);
                expecting_c2 = false;
            }

            else if *c == c1 {
                result.push(c1);
            }

            else {
                result.push(c1);
                result.push(*c);
                expecting_c2 = false;
            }
        }

        else if *c == c1 {
            expecting_c2 = true;
        }

        else {
            result.push(*c);
        }
    }

    if expecting_c2 {
        result.push(c1);
    }

    result
}
