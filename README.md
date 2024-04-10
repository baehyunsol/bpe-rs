# Byte Pair Encoding

[Byte Pair Encoding](https://en.wikipedia.org/wiki/Byte_pair_encoding) in pure Rust.

You can give inputs via stdin or files. You can run it in parallel!

---

TODO

impl file/dir compression using BPE

multi-threading: synchronization between threads would be too complicated...
how about this?

0. the main thread allocates corpus to each thread
1. each thread constructs their own dictionary just simply calling `construct_dictionary`
2. the main thread joins the dictionary
  - words with a lot of appearance are chosen
  - other words become `<UNK>`

---

TODO: tokenizer (from dictionary)
