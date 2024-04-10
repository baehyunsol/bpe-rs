use super::*;
use crate::files::read_bytes;

#[test]
fn unit_pair_roundtrip() {
    assert_eq!((0, 1), from_pair(into_pair(0, 1)));
}

#[test]
fn simple_bytes_roundtrip() {
    let sample = "This is a sample text.";
    let units = bytes_to_units(sample.as_bytes());
    let bytes = units_to_bytes(&units, &default_unit_map());

    assert_eq!(
        sample.as_bytes(),
        bytes,
    );
}

#[test]
fn assign_pair_to_new_unit_test() {
    let samples = vec![
        ("abcd abcd abab", (b'a', b'b'), b'X', "Xcd Xcd XX"),
        ("aaaa", (b'a', b'a'), b'Y', "YY"),
        ("This is an apple", (b'i', b's'), b'X', "ThX X an apple"),
    ];

    for (s, (p1, p2), new_unit, answer) in samples.into_iter() {
        let s_units = bytes_to_units(s.as_bytes());
        let answer_units = bytes_to_units(answer.as_bytes());
        let pair = into_pair(p1 as Unit, p2 as Unit);

        assert_eq!(
            assign_pair_to_new_unit(&s_units, pair, new_unit as Unit),
            answer_units,
        );
    }
}

// TODO: make it parallel
#[test]
fn dictionary_count_test() {
    dictionary_count_test_worker("./corpus/etc/1st.txt", 256);
    dictionary_count_test_worker("./corpus/etc/1st.txt", 512);
    dictionary_count_test_worker("./corpus/etc/lojban.txt", 256);
    dictionary_count_test_worker("./corpus/etc/lojban.txt", 1024);
}

fn dictionary_count_test_worker(file: &str, dictionary_size: usize) {
    let bytes = read_bytes(file).unwrap();
    let result = construct_dictionary(
        &bytes,
        DictionaryConfig::default()
            .set_dictionary_size(dictionary_size)
            .to_owned(),
    );

    let mut sum = 0;

    for (word, appearance) in result.iter() {
        sum += word.len() * appearance;
    }

    assert_eq!(bytes.len(), sum);
}
