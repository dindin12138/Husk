use bytes::BytesMut;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;
use tokio::net::UnixStream;
use tokio_util::codec::{Decoder, Encoder, LengthDelimitedCodec};

// ==========================================
// IPC Messages
// ==========================================

/// Control Plane payload sent from Client to Daemon
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
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
    /// ACK for stop-and-wait SHM expansion protocol
    SHMRemapped,
    /// ACK for distributed LRU image eviction
    EvictionAck {
        image_id: u32,
    },
    Detach,
}

/// Control Plane payload sent from Daemon to Client
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum DaemonMessage {
    HandshakeAck,
    /// Granular configuration synchronization for UI states
    ConfigSynced,
    /// Notification of screen changes
    RenderDiff {
        epoch: u64,
        shm_offset: u32,
        shm_len: u32,
    },
    /// Micro-batched notification of newly baked glyphs ready in SHM
    GlyphsReady {
        glyph_ids: Vec<u32>,
    },
    /// Signal to trigger client-side mmap re-mapping
    SHMResized {
        new_size: u64,
    },
    /// Signal to trigger client-side GPU texture destruction
    ImageEvicted {
        image_id: u32,
    },
    // TEMPORARY TRACER BULLET: Bypassing SHM to send fake pixels via UDS directly
    DummyFrame {
        epoch: u64,
        pixels: Vec<u8>,
    },
}

// ==========================================
// Security & Socket Utilities
// ==========================================

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

// ==========================================
// Codecs (Length-Prefixed Bincode)
// ==========================================

/// Codec used by the Daemon
/// Decodes: ClientMessage
/// Encodes: DaemonMessage
pub struct DaemonCodec {
    inner: LengthDelimitedCodec,
}

impl DaemonCodec {
    pub fn new() -> Self {
        Self {
            inner: LengthDelimitedCodec::new(),
        }
    }
}

impl Default for DaemonCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl Decoder for DaemonCodec {
    type Item = ClientMessage;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match self.inner.decode(src)? {
            Some(frame) => match bincode::deserialize::<ClientMessage>(&frame) {
                Ok(msg) => Ok(Some(msg)),
                Err(e) => {
                    tracing::error!("DaemonCodec bincode deserialization failed: {}", e);
                    Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
                }
            },
            None => Ok(None),
        }
    }
}

impl Encoder<DaemonMessage> for DaemonCodec {
    type Error = std::io::Error;

    fn encode(&mut self, item: DaemonMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let data = bincode::serialize(&item)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        self.inner.encode(bytes::Bytes::from(data), dst)
    }
}

/// Codec used by the Client
/// Decodes: DaemonMessage
/// Encodes: ClientMessage
pub struct ClientCodec {
    inner: LengthDelimitedCodec,
}

impl ClientCodec {
    pub fn new() -> Self {
        Self {
            inner: LengthDelimitedCodec::new(),
        }
    }
}

impl Default for ClientCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl Decoder for ClientCodec {
    type Item = DaemonMessage;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match self.inner.decode(src)? {
            Some(frame) => match bincode::deserialize::<DaemonMessage>(&frame) {
                Ok(msg) => Ok(Some(msg)),
                Err(e) => {
                    tracing::error!("ClientCodec bincode deserialization failed: {}", e);
                    Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
                }
            },
            None => Ok(None),
        }
    }
}

impl Encoder<ClientMessage> for ClientCodec {
    type Error = std::io::Error;

    fn encode(&mut self, item: ClientMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let data = bincode::serialize(&item)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        self.inner.encode(bytes::Bytes::from(data), dst)
    }
}

// ==========================================
// Tests
// ==========================================

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::{BufMut, BytesMut};

    #[test]
    fn test_ipc_simple_roundtrip() {
        let mut client_codec = ClientCodec::new();
        let mut daemon_codec = DaemonCodec::new();
        let mut buffer = BytesMut::new();

        // Client sends Handshake
        let msg_out = ClientMessage::Handshake { session_id: 1024 };
        client_codec.encode(msg_out.clone(), &mut buffer).unwrap();

        // Daemon decodes successfully
        let msg_in = daemon_codec.decode(&mut buffer).unwrap().unwrap();
        assert_eq!(msg_out, msg_in);

        // Buffer should be fully consumed
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_ipc_sticky_packets() {
        let mut daemon_codec = DaemonCodec::new();
        let mut buffer = BytesMut::new();

        // Simulate OS merging multiple messages into a single buffer
        let msg1 = DaemonMessage::HandshakeAck;
        let msg2 = DaemonMessage::RenderDiff {
            epoch: 1,
            shm_offset: 0,
            shm_len: 4096,
        };
        let msg3 = DaemonMessage::GlyphsReady {
            glyph_ids: vec![10, 20, 30],
        };

        daemon_codec.encode(msg1.clone(), &mut buffer).unwrap();
        daemon_codec.encode(msg2.clone(), &mut buffer).unwrap();
        daemon_codec.encode(msg3.clone(), &mut buffer).unwrap();

        let mut client_codec = ClientCodec::new();

        // Client must decode them sequentially and correctly
        let out1 = client_codec.decode(&mut buffer).unwrap().unwrap();
        assert_eq!(out1, msg1);

        let out2 = client_codec.decode(&mut buffer).unwrap().unwrap();
        assert_eq!(out2, msg2);

        let out3 = client_codec.decode(&mut buffer).unwrap().unwrap();
        assert_eq!(out3, msg3);

        // No more messages left
        assert!(client_codec.decode(&mut buffer).unwrap().is_none());
    }

    #[test]
    fn test_ipc_fragmented_packets() {
        let mut daemon_codec = DaemonCodec::new();
        let mut encode_buffer = BytesMut::new();

        // Construct a slightly larger message
        let msg = DaemonMessage::GlyphsReady {
            glyph_ids: (0..1000).collect(),
        };
        daemon_codec
            .encode(msg.clone(), &mut encode_buffer)
            .unwrap();

        let mut client_codec = ClientCodec::new();
        let mut decode_buffer = BytesMut::new();

        let total_bytes = encode_buffer.len();

        // Simulate extreme network conditions: receiving 1 byte at a time
        for i in 0..total_bytes {
            decode_buffer.put_u8(encode_buffer[i]);

            let result = client_codec.decode(&mut decode_buffer).unwrap();

            if i < total_bytes - 1 {
                // Should return None until the very last byte is received
                assert!(result.is_none(), "Premature decoding at byte {}", i);
            } else {
                // Successfully reassembles the full message on the final byte
                assert!(result.is_some(), "Failed to decode on final byte");
                assert_eq!(result.unwrap(), msg);
            }
        }
    }
}
