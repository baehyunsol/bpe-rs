use bpe_rs::{DictionaryConfig, construct_dictionary_from_dir};

// TEST
fn main() {
    let dict = construct_dictionary_from_dir(
        DictionaryConfig::default()
            .set_dictionary_size(16384)
            .set_dir("./corpus/wikipedia".to_string())
            .set_extension_to_read("md".to_string())
            .set_file_separator(Some(0))
            .set_ultimate_separator(Some(0))
            .set_worker_count(Some(4))
            .set_log_file(Some("./log.txt".to_string()))
            .set_dump_file(Some("./dump.sjfl".to_string()))
            .to_owned()
    ).unwrap();
}
