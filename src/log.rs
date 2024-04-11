use crate::files::{
    exists,
    write_string,
    FileError,
    WriteMode,
};
use chrono::offset::Local;

pub fn initialize_log_file(path: &str, remove_existing_file: bool) -> Result<(), FileError> {
    if !exists(path) || remove_existing_file {
        write_string(path, "", WriteMode::CreateOrTruncate)?;
    }

    Ok(())
}

pub fn write_log(
    path: Option<String>,
    owner: &str,
    msg: &str,
) {
    if let Some(path) = path {
        write_string(
            &path,
            &format!(
                "{} | {} | {msg}\n",
                Local::now().to_rfc2822(),
                if owner.len() < 16 {
                    format!("{}{owner}", " ".repeat(16 - owner.len()))
                } else {
                    owner.to_string()
                },
            ),
            WriteMode::AlwaysAppend,
        ).unwrap();
    }
}
