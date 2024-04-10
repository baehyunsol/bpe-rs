use bpe_rs::{DictionaryConfig, construct_dictionary_from_dir};

fn main() {
    let dict = construct_dictionary_from_dir(
        DictionaryConfig::default()
            .set_dictionary_size(4096)
            .set_dir("../ggb2_prj/ggb2/data".to_string())
            .set_extension_to_read("sjfl".to_string())
            .set_file_separator(Some(0))
            .set_ultimate_separator(Some(0))
            .to_owned()
    ).unwrap();

    println!("{dict:?}");
}
