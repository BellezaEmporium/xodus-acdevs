// Hardware probing utilities

use std::io;
#[cfg(target_os = "linux")]
use std::process::{Command, Stdio};

use base64::prelude::*;
#[cfg(any(target_os = "macos", target_os = "ios", target_family = "windows"))]
use smbioslib::raw_smbios_from_device;
#[cfg(not(target_os = "linux"))]
use smbioslib::{SMBiosSystemInformation, SystemUuidData, table_load_from_device};

use crate::clep;
use crate::models::devicecredential::Component;

pub fn probe_provision_components() -> Vec<Component> {
    let mut components = Vec::with_capacity(16);
    let drive_serial = BASE64_STANDARD.decode("AA==").unwrap();
    let mut smbios_buf = [0; 256];
    let mut drive_buf = [0; 64];

    let smbios = load_raw_smbios().ok();
    let parsed_smbios = load_smbios_fields(smbios.as_deref()).ok();

    drive_buf
        .iter_mut()
        .zip(drive_serial.iter())
        .for_each(|(place, data)| *place = *data);
    if let Some(smbios) = smbios.as_ref() {
        smbios_buf
            .iter_mut()
            .zip(smbios.iter())
            .for_each(|(place, data)| *place = *data);
    }
    let (clepv2, clepv4) = clep::challenge::get_license_challange(smbios_buf, drive_buf);

    components.push(Component::new(4113, "AA==".to_string()));
    components.push(Component::error(4101));
    components.push(Component::new(8196, BASE64_STANDARD.encode(clepv2)));
    components.push(Component::new(8197, BASE64_STANDARD.encode(clepv4)));

    if let Some((version, serial, uuid)) = parsed_smbios {
        components.push(Component::new(4100, BASE64_STANDARD.encode(version)));
        components.push(Component::new(4101, BASE64_STANDARD.encode(serial)));
        components.push(Component::new(4102, BASE64_STANDARD.encode(uuid)));
    } else {
        components.push(Component::error(4100));
        components.push(Component::error(4101));
        components.push(Component::error(4102));
    }

    components.push(Component::new(4145, "AQAAAA==".to_string()));
    components.push(Component::error(4160));
    components.push(Component::error(4161));

    // Common values sent with the request
    // "4128"
    // "4130"
    // "4112"
    // "4113"
    // "4098"
    // "4099"
    // "4100"
    // "4101"
    // "4102"
    // "4097"
    // "8195"
    // "8196"
    // "8197"
    // "4144"
    // "4145"
    // "4160"
    // "4161"

    components
}

#[cfg(target_os = "linux")]
fn load_smbios_fields(raw: Option<&[u8]>) -> io::Result<(Vec<u8>, Vec<u8>, [u8; 16])> {
    let smbios =
        raw.ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "missing raw SMBIOS data"))?;
    parse_smbios(smbios)
}

#[cfg(not(target_os = "linux"))]
fn load_smbios_fields(_raw: Option<&[u8]>) -> io::Result<(Vec<u8>, Vec<u8>, [u8; 16])> {
    let data = table_load_from_device()?;
    let system_info = data
        .first::<SMBiosSystemInformation>()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "missing SMBIOS Type 1"))?;

    let version = system_info
        .version()
        .to_utf8_lossy()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing SMBIOS version"))?
        .into_bytes();
    let serial = system_info
        .serial_number()
        .to_utf8_lossy()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing SMBIOS serial"))?
        .into_bytes();
    let uuid = match system_info.uuid() {
        Some(SystemUuidData::Uuid(uuid)) => uuid.raw,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "missing SMBIOS UUID",
            ));
        }
    };

    Ok((version, serial, uuid))
}

#[cfg(target_os = "linux")]
fn parse_smbios(smbios: &[u8]) -> io::Result<(Vec<u8>, Vec<u8>, [u8; 16])> {
    if smbios.len() < 24 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "SMBIOS entry shorter than the 24-byte fixed header",
        ));
    }

    let length = smbios[1] as usize;
    let version = smbios[6];
    let serial = smbios[7];
    let uuid: [u8; 16] = smbios[8..24]
        .try_into()
        .expect("range checked to be exactly 16 bytes above");

    if length >= smbios.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "SMBIOS string table offset points past the end of the entry",
        ));
    }
    let stringsbuf = &smbios[length..];
    let mut strings: Vec<&[u8]> = Vec::new();
    strings.push(&[]);
    let mut cursor = 0;
    while cursor < stringsbuf.len() {
        let end = stringsbuf[cursor..]
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(stringsbuf.len() - cursor)
            + cursor;
        let slice = &stringsbuf[cursor..end];
        strings.push(slice);
        cursor = end + 1;
        if cursor >= stringsbuf.len() || stringsbuf[cursor] == 0 {
            break;
        }
    }

    let pick_string = |index: u8| -> io::Result<Vec<u8>> {
        strings
            .get(index as usize)
            .map(|s| s.to_vec())
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "SMBIOS string index out of range",
                )
            })
    };

    Ok((pick_string(version)?, pick_string(serial)?, uuid))
}

#[cfg(any(target_os = "macos", target_os = "ios", target_family = "windows"))]
fn load_raw_smbios() -> io::Result<Vec<u8>> {
    raw_smbios_from_device()
}

#[cfg(target_os = "linux")]
fn load_raw_smbios() -> io::Result<Vec<u8>> {
    use std::io::Read;
    use std::time::{Duration, Instant};

    const PROBE_TIMEOUT: Duration = Duration::from_secs(30);
    const POLL_INTERVAL: Duration = Duration::from_millis(50);

    let mut cmd = Command::new("pkexec")
        .args(["cat", "/sys/firmware/dmi/entries/1-0/raw"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    // Drain stdout concurrently so a payload larger than the pipe buffer
    // cannot deadlock against our polling loop.
    let mut piped_stdout = cmd.stdout.take().expect("stdout was just piped");
    let reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = piped_stdout.read_to_end(&mut buf);
        buf
    });

    let deadline = Instant::now() + PROBE_TIMEOUT;
    let mut status = None;
    while Instant::now() < deadline {
        match cmd.try_wait()? {
            Some(exit) => {
                status = Some(exit);
                break;
            }
            None => std::thread::sleep(POLL_INTERVAL),
        }
    }

    if status.is_none() {
        let _ = cmd.kill();
        let _ = cmd.wait();
        let _ = reader.join();
        return Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "pkexec did not finish within 30s (no polkit agent available?)",
        ));
    }

    let stdout = reader
        .join()
        .map_err(|_| io::Error::other("SMBIOS reader thread panicked"))?;

    if status.expect("status was set in the loop above").success() {
        Ok(stdout)
    } else {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "unable to probe SMBIOS data",
        ))
    }
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "ios",
    target_family = "windows"
)))]
fn load_raw_smbios() -> io::Result<Vec<u8>> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "raw SMBIOS loading is unsupported on this platform",
    ))
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::parse_smbios;

    /// Builds a synthetic Type 1 entry: 24-byte fixed header followed by a
    /// null-terminated string table with a trailing double null.
    fn sample_entry(version_index: u8, serial_index: u8) -> Vec<u8> {
        let mut entry = vec![0u8; 24];
        entry[0] = 1; // Type 1 (System Information)
        entry[1] = 24; // string area starts right after the header
        entry[6] = version_index;
        entry[7] = serial_index;
        entry[8..24].copy_from_slice(&0u128.to_le_bytes()); // UUID
        entry.extend_from_slice(b"vBox 7.1\0"); // string 1
        entry.extend_from_slice(b"SN-123456\0"); // string 2
        entry.push(0); // terminating double null
        entry
    }

    #[test]
    fn parses_version_serial_and_uuid() {
        let entry = sample_entry(1, 2);
        let (version, serial, uuid) = parse_smbios(&entry).unwrap();
        assert_eq!(version, b"vBox 7.1");
        assert_eq!(serial, b"SN-123456");
        assert_eq!(uuid, 0u128.to_le_bytes());
    }

    #[test]
    fn index_zero_maps_to_empty_string() {
        let entry = sample_entry(0, 0);
        let (version, serial, _uuid) = parse_smbios(&entry).unwrap();
        assert!(version.is_empty());
        assert!(serial.is_empty());
    }

    #[test]
    fn rejects_truncated_entry() {
        let entry = sample_entry(1, 2);
        let err = parse_smbios(&entry[..12]).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn rejects_string_table_offset_out_of_range() {
        let mut entry = sample_entry(1, 2);
        entry[1] = 200; // string area starts beyond the entry end
        let err = parse_smbios(&entry).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn rejects_string_index_out_of_range() {
        let entry = sample_entry(5, 2); // only 2 strings exist
        let err = parse_smbios(&entry).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }
}
