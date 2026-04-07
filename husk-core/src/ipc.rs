use bytes::BytesMut;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;
use tokio::net::UnixStream;
use tokio_util::codec::{Decoder, Encoder, LengthDelimitedCodec};

/// Control Plane payload sent from Client to Daemon
#[derive(Serialize, Deserialize, Debug)]
pub enum ClientMessage {
    Handshake {
        session_id: u32,
    },
    Input(Vec<u8>),
    /// Epoch is utilized by the Daemon to preemptively drop stale render tasks
    Resize {
        cols: u16,
        rows: u16,
        px_width: u32,
        px_height: u32,
        epoch: u64,
    },
    Detach,
}

/// Control Plane payload sent from Daemon to Client
#[derive(Serialize, Deserialize, Debug)]
pub enum DaemonMessage {
    HandshakeAck,
    /// Notification of screen changes.
    /// Actual pixel data is passed via Data Plane (Shared Memory)
    RenderDiff {
        epoch: u64,
        shm_offset: u32,
        shm_len: u32,
    },
    FontAtlasUpdated,
}

/// Computes a secure, user-isolated path for the Unix Domain Socket
pub fn get_socket_path() -> PathBuf {
    let uid = unsafe { libc::geteuid() };

    let base_dir = if cfg!(target_os = "linux") {
        env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| format!("/tmp/husk-{}", uid))
    } else if cfg!(target_os = "macos") {
        env::var("TMPDIR").unwrap_or_else(|_| format!("/tmp/husk-{}", uid))
    } else {
        panic!("Husk is only supported on Linux and macOS");
    };

    PathBuf::from(base_dir).join(format!("husk-daemon-{}.sock", uid))
}

/// Core security boundary: validates that the connecting peer shares the same OS UID.
/// Mitigates local privilege escalation attacks.
pub fn verify_peer_credentials(stream: &UnixStream) -> std::io::Result<()> {
    let cred = stream.peer_cred()?;
    let expected_uid = unsafe { libc::geteuid() };

    if cred.uid() != expected_uid {
        tracing::error!(
            "Security breach attempt! Peer UID: {}, Expected UID: {}",
            cred.uid(),
            expected_uid
        );
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "UID mismatch: privilege escalation denied",
        ));
    }

    tracing::debug!("IPC: Peer credentials verified (UID: {})", expected_uid);
    Ok(())
}

/// Husk IPC Codec combining Bincode serialization with Length-Prefixed Framing
/// to completely eliminate TCP/UDS sticky packets.
pub struct HuskCodec {
    inner: LengthDelimitedCodec,
}

impl HuskCodec {
    pub fn new() -> Self {
        Self {
            inner: LengthDelimitedCodec::new(),
        }
    }
}

impl Default for HuskCodec {
    fn default() -> Self {
        Self::new()
    }
}

// Decodes the byte stream into ClientMessage (Used by the Daemon)
impl Decoder for HuskCodec {
    type Item = ClientMessage;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match self.inner.decode(src)? {
            Some(frame) => match bincode::deserialize::<ClientMessage>(&frame) {
                Ok(msg) => Ok(Some(msg)),
                Err(e) => {
                    tracing::error!("Bincode deserialization failed: {}", e);
                    Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
                }
            },
            None => Ok(None),
        }
    }
}

// Encodes DaemonMessage into the byte stream (Used by the Daemon)
// Note: Client implementation will require the inverse Encoder/Decoder pair.
impl Encoder<DaemonMessage> for HuskCodec {
    type Error = std::io::Error;

    fn encode(&mut self, item: DaemonMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let data = bincode::serialize(&item)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        self.inner.encode(bytes::Bytes::from(data), dst)
    }
}
