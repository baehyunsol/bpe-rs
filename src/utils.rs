pub fn prettify_file_size(bytes: u64) -> String {
    if bytes < (1 << 10) {
        format!("{bytes}B")
    }

    else if bytes < (1 << 20) {
        format!("{}kiB", bytes >> 10)
    }

    else if bytes < (1 << 30) {
        format!("{}MiB", bytes >> 20)
    }

    else {
        format!("{}GiB", bytes >> 30)
    }
}
