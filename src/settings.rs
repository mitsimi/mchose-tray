use std::{
    ffi::OsString,
    fs::{self, OpenOptions},
    io::{self, Write},
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
};
use windows_sys::Win32::Storage::FileSystem::{
    MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum DisplayMode {
    #[default]
    Percentage,
    Battery,
}

impl DisplayMode {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "percentage" => Some(Self::Percentage),
            "battery" => Some(Self::Battery),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Percentage => "percentage",
            Self::Battery => "battery",
        }
    }
}

pub fn directory() -> PathBuf {
    directory_from(
        std::env::var_os("MCHOSE_TRAY_DATA_DIR"),
        std::env::var_os("LOCALAPPDATA"),
    )
}

fn directory_from(override_dir: Option<OsString>, local_app_data: Option<OsString>) -> PathBuf {
    if let Some(path) = override_dir {
        return path.into();
    }
    local_app_data
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("MchoseTray")
}

pub fn load() -> DisplayMode {
    load_from(&directory())
}

fn load_from(directory: &Path) -> DisplayMode {
    fs::read_to_string(directory.join("display.txt"))
        .ok()
        .and_then(|s| DisplayMode::parse(&s))
        .unwrap_or_default()
}

pub fn save(mode: DisplayMode) -> io::Result<()> {
    save_to(&directory(), mode)
}

fn save_to(directory: &Path, mode: DisplayMode) -> io::Result<()> {
    fs::create_dir_all(directory)?;
    let destination = directory.join("display.txt");
    let temporary = directory.join(format!("display.{}.tmp", std::process::id()));
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&temporary)?;
    file.write_all(mode.as_str().as_bytes())?;
    file.sync_all()?;
    drop(file);

    let from: Vec<u16> = temporary.as_os_str().encode_wide().chain(Some(0)).collect();
    let to: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    // SAFETY: both paths are terminated, remain alive for the call, and name
    // files in the same directory so replacement is atomic.
    let moved = unsafe {
        MoveFileExW(
            from.as_ptr(),
            to.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if moved == 0 {
        let error = io::Error::last_os_error();
        let _ = fs::remove_file(temporary);
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preferences_round_trip_and_reject_invalid_values() {
        for mode in [DisplayMode::Percentage, DisplayMode::Battery] {
            assert_eq!(DisplayMode::parse(mode.as_str()), Some(mode));
        }
        assert_eq!(DisplayMode::parse("garbage"), None);
        assert_eq!(
            DisplayMode::parse(" battery\r\n"),
            Some(DisplayMode::Battery)
        );
    }

    #[test]
    fn data_directory_can_be_overridden() {
        assert_eq!(
            directory_from(
                Some(OsString::from("test-data")),
                Some(OsString::from("ignored"))
            ),
            PathBuf::from("test-data")
        );
    }

    #[test]
    fn settings_are_atomically_replaced_in_an_isolated_directory() {
        let directory = std::env::temp_dir().join(format!(
            "mchose-tray-settings-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        save_to(&directory, DisplayMode::Battery).unwrap();
        assert_eq!(load_from(&directory), DisplayMode::Battery);
        save_to(&directory, DisplayMode::Percentage).unwrap();
        assert_eq!(load_from(&directory), DisplayMode::Percentage);
        assert!(
            !directory
                .join(format!("display.{}.tmp", std::process::id()))
                .exists()
        );
        fs::remove_file(directory.join("display.txt")).unwrap();
        fs::remove_dir(directory).unwrap();
    }
}
