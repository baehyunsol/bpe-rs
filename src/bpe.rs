use crate::dictionary::{Dictionary, DictionaryConfig};
use crate::files::{FileError, WriteMode, extension, file_size, read_dir, write_string};
use crate::log::{initialize_log_file, write_log};
use crate::multi::{MessageFromMain, MessageToMain, init_channels};
use crate::utils::prettify_file_size;
use smallvec::{SmallVec, smallvec};
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::TryRecvError;
use std::thread::sleep;
use std::time::Duration;

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

pub fn construct_dictionary_from_dir(config: DictionaryConfig) -> Result<Dictionary, FileError> {
    if let Some(path) = &config.write_log_at {
        initialize_log_file(path, true).unwrap();
    }

    write_log(
        config.write_log_at.clone(),
        "master",
        "Hello from master!",
    );

    let channels = init_channels(
        config.parallel_worker_count.unwrap_or_else(
            || std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1).max(1)
        ),
        &config,
    );

    let files = read_dir(&config.dir_option.path)?;
    let ext_to_see = Some(config.dir_option.ext.clone());

    let mut files_with_sizes = files.iter().filter(
        |file| match (extension(file), file_size(file)) {
            (Ok(ext), Ok(_size)) if ext == ext_to_see => true,
            _ => false,
        }
    ).map(
        |file| (file.to_string(), file_size(file).unwrap())
    ).collect::<Vec<_>>();

    files_with_sizes.sort_by_key(|(_, size)| *size);

    write_log(
        config.write_log_at.clone(),
        "master",
        &format!(
            "finished sorting files: got {} files to see (total size {})",
            files_with_sizes.len(),
            prettify_file_size(files_with_sizes.iter().map(|(_, size)| *size).sum::<u64>()),
        ),
    );

    let mut worker_index = 0;
    let mut done = 0;
    let mut file_index = 0;

    loop {
        let mut files_to_read = vec![];
        let mut curr_chunk_size = 0;

        // TODO: divide big files
        while curr_chunk_size < config.dir_option.file_chunk_size && file_index < files_with_sizes.len() {
            files_to_read.push(files_with_sizes[file_index].0.clone());
            curr_chunk_size += files_with_sizes[file_index].1 as usize;
            file_index += 1;
        }

        write_log(
            config.write_log_at.clone(),
            "master",
            &format!(
                "gave jobs to a worker: {} files (total size {})",
                files_to_read.len(),
                prettify_file_size(curr_chunk_size as u64),
            ),
        );

        channels[worker_index % channels.len()].send(
            MessageFromMain::ReadTheseFiles(files_to_read)
        ).unwrap();

        if file_index == files_with_sizes.len() {
            break;
        }

        worker_index += 1;
    }

    let mut result = Dictionary::empty();

    loop {
        let mut has_update = false;

        for channel in channels.iter() {
            match channel.try_recv() {
                Ok(msg) => match msg {
                    MessageToMain::NewDictionary(dictionary) => {
                        has_update = true;
                        result.merge(&dictionary);
                    },
                    MessageToMain::Done => {
                        done += 1;
                    },
                },
                Err(TryRecvError::Disconnected) => {
                    // TODO: what do I do here?
                    // There are 2 cases
                    // 1, this worker has done its job
                    // 2, this worker has an error
                },
                _ => {
                    // nop
                },
            }
        }

        if let Some(path) = &config.dump_result_at {
            if has_update {
                write_string(
                    path,
                    &format!("{result:?}"),
                    WriteMode::CreateOrTruncate,
                ).unwrap();

                write_log(
                    config.write_log_at.clone(),
                    "master",
                    &format!("dumped result at {path}")
                );
            }
        }

        sleep(Duration::from_millis(2000));

        if done == channels.len() {
            break;
        }
    }

    write_log(
        config.write_log_at.clone(),
        "master",
        "Goodbye from master!",
    );

    Ok(result)
}

pub fn construct_dictionary(
    bytes: &[u8],
    config: DictionaryConfig,
) -> Dictionary {
    if let Some(path) = &config.write_log_at {
        initialize_log_file(path, false).unwrap();
    }

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
