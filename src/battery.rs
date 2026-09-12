use hidapi::HidApi;
use std::time::{Duration, Instant};

pub const MODEL: &str = "MCHOSE K7 V2 Ultra+";
const RECEIVER_VID: u16 = 0x3837;
const RECEIVER_PID: u16 = 0x1014;
const MOUSE_VID: [u8; 2] = [0x37, 0x38];
const TIMEOUT: Duration = Duration::from_millis(800);
const READ_SLICE: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Checking,
    Disconnected,
    Unavailable,
    Battery { percent: u8, charging: bool },
}

impl Status {
    pub fn label(self) -> String {
        match self {
            Self::Checking => "Checking battery…".into(),
            Self::Disconnected => "Receiver disconnected".into(),
            Self::Unavailable => "Battery unavailable — mouse may be asleep".into(),
            Self::Battery { percent, charging } => {
                let suffix = if charging && percent == 100 {
                    " · Fully charged"
                } else if charging {
                    " · Charging"
                } else {
                    ""
                };
                format!("{percent}%{suffix}")
            }
        }
    }
}

pub fn request() -> [u8; 64] {
    let mut report = [0; 64];
    report[..9].copy_from_slice(&[0x4d, 1, 1, 0, 0, 9, 0, 0, 8]);
    report
}

pub fn parse(report: &[u8]) -> Option<Status> {
    // Sequence (byte 7) is echoed by firmware and may change in future versions.
    if report.len() < 9 || report[..3] != [0x4d, 1, 1] || report[4..7] != [0, 9, 0] {
        return None;
    }
    let length = usize::from(report[3]);
    if length < 14 || report.len() < 9 + length {
        return None;
    }
    let checksum = report[2..8 + length].iter().fold(0, |acc, x| acc ^ x);
    if checksum != report[8 + length] {
        return None;
    }
    let payload = &report[8..8 + length];
    if payload[..2] != MOUSE_VID {
        return None;
    }
    match payload[10] {
        0 => return Some(Status::Unavailable),
        2 => {}
        _ => return None,
    }
    let (percent, charge) = (payload[12], payload[11]);
    if percent > 100 || charge > 1 {
        return None;
    }
    Some(Status::Battery {
        percent,
        charging: charge == 1,
    })
}

pub fn read() -> Result<Status, String> {
    read_with_logger(|_| {})
}

pub fn read_with_logger(mut logger: impl FnMut(&str)) -> Result<Status, String> {
    // Re-enumerate each time: handles do not survive receiver reconnection.
    let api = HidApi::new().map_err(|e| e.to_string())?;
    let mut receivers: Vec<_> = api
        .device_list()
        .filter(|device| {
            device.vendor_id() == RECEIVER_VID
                && device.product_id() == RECEIVER_PID
                && device.usage_page() == 0xff01
                && device.usage() == 1
                && device.interface_number() == 2
        })
        .collect();
    if receivers.is_empty() {
        return Ok(Status::Disconnected);
    }
    // Prefer the known name, but keep the protocol selectors as a fallback for
    // harmless firmware string changes. Try every matching receiver.
    receivers.sort_by_key(|device| device.product_string() != Some(MODEL));
    let mut fallback = None;
    let mut errors = Vec::new();
    for info in receivers {
        let product = info.product_string().unwrap_or("<unknown>");
        if product != MODEL {
            logger(&format!(
                "Receiver product string is {product:?}; trying the verified protocol"
            ));
        }
        match read_device(&api, info) {
            Ok(status @ Status::Battery { .. }) => return Ok(status),
            Ok(status) => fallback = Some(status),
            Err(error) => errors.push(format!("{product}: {error}")),
        }
    }
    if let Some(status) = fallback {
        Ok(status)
    } else {
        Err(errors.join("; "))
    }
}

fn read_device(api: &HidApi, info: &hidapi::DeviceInfo) -> Result<Status, String> {
    let device = info.open_device(api).map_err(|e| e.to_string())?;
    if device.write(&request()).map_err(|e| e.to_string())? != 64 {
        return Err("Incomplete battery request".into());
    }
    let deadline = Instant::now() + TIMEOUT;
    while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
        let mut report = [0; 64];
        let wait = remaining.min(READ_SLICE);
        let count = device
            .read_timeout(&mut report, wait.as_millis().max(1) as i32)
            .map_err(|e| e.to_string())?;
        if let Some(status) = parse(&report[..count]) {
            return Ok(status);
        }
    }
    Ok(Status::Unavailable)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capture() -> Vec<u8> {
        // Real receiver, 83%, wireless, not charging. Includes packet checksum.
        vec![
            0x4d, 1, 1, 0x10, 0, 9, 0, 0, 0x37, 0x38, 0x26, 0x40, 4, 0, 0, 0, 0, 0x10, 2, 0, 0x53,
            0, 8, 0xe4, 0xd8,
        ]
    }

    fn changed(connection: u8, charge: u8, percent: u8) -> Vec<u8> {
        let mut r = capture();
        r[18] = connection;
        r[19] = charge;
        r[20] = percent;
        r[24] = r[2..24].iter().fold(0, |acc, x| acc ^ x);
        r
    }

    #[test]
    fn real_capture() {
        assert_eq!(
            parse(&capture()),
            Some(Status::Battery {
                percent: 83,
                charging: false
            })
        );
    }

    #[test]
    fn no_cached_percentage_when_link_is_down() {
        assert_eq!(parse(&changed(0, 0, 83)), Some(Status::Unavailable));
    }

    #[test]
    fn rejects_corruption_truncation_and_other_commands() {
        let r = capture();
        for end in 0..r.len() {
            assert_eq!(parse(&r[..end]), None);
        }
        for index in 0..r.len() {
            if index == 7 {
                continue;
            }
            let mut broken = r.clone();
            broken[index] ^= 1;
            assert_eq!(parse(&broken), None, "offset {index}");
        }
        assert_eq!(parse(&changed(2, 0, 101)), None);
        assert_eq!(parse(&changed(2, 2, 80)), None);
        assert_eq!(parse(&changed(1, 0, 80)), None);
    }

    #[test]
    fn accepts_nonzero_sequence_when_checksum_matches() {
        let mut r = capture();
        r[7] = 7;
        r[24] = r[2..24].iter().fold(0, |acc, x| acc ^ x);
        assert_eq!(
            parse(&r),
            Some(Status::Battery {
                percent: 83,
                charging: false
            })
        );
    }

    #[test]
    fn zero_and_full_are_valid() {
        assert_eq!(
            parse(&changed(2, 0, 0)),
            Some(Status::Battery {
                percent: 0,
                charging: false
            })
        );
        assert_eq!(
            parse(&changed(2, 1, 100)),
            Some(Status::Battery {
                percent: 100,
                charging: true
            })
        );
    }

    #[test]
    fn request_is_only_device_info() {
        let r = request();
        assert_eq!(&r[..8], &[0x4d, 1, 1, 0, 0, 9, 0, 0]);
        assert_eq!(r[8], r[2..8].iter().fold(0, |acc, x| acc ^ x));
        assert!(r[9..].iter().all(|x| *x == 0));
    }
}
