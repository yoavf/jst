use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const INSTALLATION_ID_FILE: &str = "installation-id";

pub fn installation_id() -> io::Result<String> {
    if let Ok(value) = std::env::var("JST_INSTALLATION_ID") {
        return validate(value);
    }

    load_or_create(&config_directory()?.join(INSTALLATION_ID_FILE))
}

fn config_directory() -> io::Result<PathBuf> {
    if let Some(path) = std::env::var_os("JST_CONFIG_DIR") {
        return Ok(PathBuf::from(path));
    }
    if let Some(path) = std::env::var_os("XDG_CONFIG_HOME") {
        return Ok(PathBuf::from(path).join("jst"));
    }
    if let Some(path) = std::env::var_os("HOME") {
        return Ok(PathBuf::from(path).join(".config/jst"));
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "cannot determine JST config directory",
    ))
}

fn load_or_create(path: &Path) -> io::Result<String> {
    match fs::read_to_string(path) {
        Ok(value) => return validate(value),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }

    let parent = path.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "invalid installation ID path")
    })?;
    fs::create_dir_all(parent)?;
    let value = generate_id()?;

    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    match options.open(path) {
        Ok(mut file) => {
            file.write_all(value.as_bytes())?;
            Ok(value)
        }
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            validate(fs::read_to_string(path)?)
        }
        Err(error) => Err(error),
    }
}

fn generate_id() -> io::Result<String> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|error| {
        io::Error::other(format!("failed to generate installation ID: {error}"))
    })?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;

    let hex = bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    ))
}

fn validate(value: String) -> io::Result<String> {
    let value = value.trim().to_ascii_lowercase();
    let valid = value.len() == 36
        && value.chars().enumerate().all(|(index, character)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                character == '-'
            } else {
                character.is_ascii_hexdigit()
            }
        });

    if valid {
        Ok(value)
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid JST installation ID",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{generate_id, load_or_create, validate};
    use std::fs;

    #[test]
    fn generates_and_reuses_anonymous_id() {
        let directory =
            std::env::temp_dir().join(format!("jst-installation-test-{}", generate_id().unwrap()));
        let path = directory.join("installation-id");

        let first = load_or_create(&path).unwrap();
        let second = load_or_create(&path).unwrap();

        assert_eq!(first, second);
        assert!(validate(first).is_ok());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn rejects_malformed_ids() {
        assert!(validate("not-an-id".to_string()).is_err());
    }
}
