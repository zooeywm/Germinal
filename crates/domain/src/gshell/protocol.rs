/// GNative PTY-side OSC request prefix.
///
/// Format:
/// ESC ] 777 ; gnative = <app-name> BEL
pub const GNATIVE_REQUEST_PREFIX: &[u8] = b"\x1b]777;gnative=";

/// GNative PTY-side OSC request terminator.
pub const GNATIVE_REQUEST_TERMINATOR: u8 = b'\x07';

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GNativeAppName(String);

impl GNativeAppName {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GRequest {
    EnterGNative { app: GNativeAppName },
}

pub fn detect_gnative_request(bytes: &[u8]) -> Option<GRequest> {
    let prefix_start = bytes
        .windows(GNATIVE_REQUEST_PREFIX.len())
        .position(|window| window == GNATIVE_REQUEST_PREFIX)?;

    let app_start = prefix_start + GNATIVE_REQUEST_PREFIX.len();

    let app_len = bytes[app_start..]
        .iter()
        .position(|byte| *byte == GNATIVE_REQUEST_TERMINATOR)?;

    let app_bytes = &bytes[app_start..app_start + app_len];

    if app_bytes.is_empty() {
        return None;
    }

    let app_name = std::str::from_utf8(app_bytes).ok()?;

    Some(GRequest::EnterGNative {
        app: GNativeAppName::new(app_name),
    })
}
