#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DictionaryConfig {
    /// It guarantees that the result of `construct_dictionary` is smaller than or equal to `dictionary_size`.
    pub dictionary_size: usize,

    /// If it's true, the result dictionary contains single byte tokens even though they do not appear.
    pub keep_single_byte_tokens: bool,

    /// It does not make an entry for tokens that appears less than `minimum_appearance` times.
    /// Even though this value is set, the result dictionary may contain entries whose `appearance` is less than this value.
    pub minimum_appearance: Option<usize>,

    /// This byte is never included in any multi-byte token.
    pub ultimate_separator: Option<u8>,

    /// It's ignored if you're constructing a dictionary from raw input.
    pub dir_option: DirOption,
}

impl DictionaryConfig {
    pub fn set_dictionary_size(&mut self, size: usize) -> &mut Self {
        self.dictionary_size = size;

        self
    }

    pub fn set_keep_single_byte_tokens(&mut self, keep: bool) -> &mut Self {
        self.keep_single_byte_tokens = keep;

        self
    }

    pub fn set_minimum_appearance(&mut self, minimum: Option<usize>) -> &mut Self {
        self.minimum_appearance = minimum;

        self
    }

    pub fn set_ultimate_separator(&mut self, separator: Option<u8>) -> &mut Self {
        self.ultimate_separator = separator;

        self
    }

    pub fn set_dir(&mut self, dir: String) -> &mut Self {
        self.dir_option.path = dir;

        self
    }

    pub fn set_extension_to_read(&mut self, ext: String) -> &mut Self {
        self.dir_option.ext = ext;

        self
    }

    pub fn set_file_chunk_size(&mut self, size: usize) -> &mut Self {
        self.dir_option.file_chunk_size = size;

        self
    }

    pub fn set_file_separator(&mut self, separator: Option<u8>) -> &mut Self {
        self.dir_option.file_separator = separator;

        self
    }
}

impl Default for DictionaryConfig {
    fn default() -> Self {
        DictionaryConfig {
            dictionary_size: 2048,
            keep_single_byte_tokens: true,
            minimum_appearance: Some(3),
            ultimate_separator: None,
            dir_option: DirOption::default(),
        }
    }
}

/// It reads all the files with the given extension, in the given path.
/// It does NOT search recursively.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DirOption {
    pub path: String,
    pub ext: String,

    /// (in bytes)\
    /// If there're multiple small files, it concats them until the total size is greater than this value.
    /// If there're big files, it divides them into chunks with this size.
    pub file_chunk_size: usize,

    /// when files are joined, this character is used as a separator
    pub file_separator: Option<u8>,
}

impl Default for DirOption {
    fn default() -> Self {
        DirOption {
            path: String::new(),
            ext: String::new(),
            file_chunk_size: 8 * 1024 * 1024,  // 8 MiB
            file_separator: None,
        }
    }
}
