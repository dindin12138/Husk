use futures::{SinkExt, StreamExt};
use husk_core::ipc::{
    ClientMessage, DaemonCodec, DaemonMessage, get_socket_path, verify_peer_credentials,
};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use tokio::net::{UnixListener, UnixStream};
use tokio_util::codec::Framed;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    tracing::info!("Husk Daemon starting...");

    let socket_path = get_socket_path();
    run_server(&socket_path).await?;

    Ok(())
}

/// Binds the UDS listener and applies strict OS-level permissions
pub async fn run_server(socket_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Remove existing socket file if it crashed previously
    if socket_path.exists() {
        fs::remove_file(socket_path)?;
        tracing::debug!("Removed stale socket file at {:?}", socket_path);
    }

    let listener = UnixListener::bind(socket_path)?;

    // Strict security boundary: 0o600 ensures only the owner can read/write
    let mut perms = fs::metadata(socket_path)?.permissions();
    perms.set_mode(0o600);
    fs::set_permissions(socket_path, perms)?;

    tracing::info!("Daemon listening securely on {:?}", socket_path);

    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                // Secondary security boundary: OS-level UID verification
                if let Err(e) = verify_peer_credentials(&stream) {
                    tracing::warn!("Rejecting connection: {}", e);
                    continue;
                }

                tracing::info!("Client connected securely. Spawning handler.");
                tokio::spawn(async move {
                    if let Err(e) = handle_client(stream).await {
                        tracing::error!("Client connection terminated with error: {}", e);
                    }
                });
            }
            Err(e) => {
                tracing::error!("Failed to accept incoming connection: {}", e);
            }
        }
    }
}

/// Handles the lifecycle of a single authenticated client connection
async fn handle_client(stream: UnixStream) -> Result<(), Box<dyn std::error::Error>> {
    let mut framed = Framed::new(stream, DaemonCodec::new());

    // Wait for strict Handshake sequence
    if let Some(Ok(ClientMessage::Handshake { session_id })) = framed.next().await {
        tracing::info!("Handshake received for Session ID: {}", session_id);
        framed.send(DaemonMessage::HandshakeAck).await?;
        tracing::debug!("HandshakeAck sent.");
    } else {
        return Err("Invalid or missing Handshake. Dropping client.".into());
    }

    // Dummy State Machine Loop
    let mut current_epoch = 0;

    while let Some(message_result) = framed.next().await {
        match message_result? {
            ClientMessage::Input(data) => {
                tracing::debug!("Received input bytes: {:?}", data);

                // Dummy render logic: Generate a fake 4x4 RGBA pixel block (64 bytes)
                // This will eventually trigger the real libghostty-vt state machine
                let fake_pixels = vec![255; 64];

                let diff_msg = DaemonMessage::DummyFrame {
                    epoch: current_epoch,
                    pixels: fake_pixels,
                };

                framed.send(diff_msg).await?;
                tracing::debug!("Sent DummyFrame for epoch {}", current_epoch);
            }
            ClientMessage::Resize { epoch, .. } => {
                tracing::info!("Resize received. Updating epoch to {}", epoch);
                current_epoch = epoch;
            }
            ClientMessage::Detach => {
                tracing::info!("Client requested clean detach. Closing session.");
                break;
            }
            _ => {
                tracing::debug!("Received unhandled message variant in Dummy state machine.");
            }
        }
    }

    tracing::info!("Client handler loop exited cleanly.");
    Ok(())
}

// ==========================================
// Tests
// ==========================================

#[cfg(test)]
mod tests {
    use super::*;
    use husk_core::ipc::ClientCodec;
    use tokio::time::{Duration, sleep};

    #[tokio::test]
    async fn test_daemon_dummy_flow() {
        let socket_path = Path::new("/tmp/husk-daemon-test-flow.sock");

        // Spawn Daemon in the background
        let path_clone = socket_path.to_path_buf();
        let daemon_handle = tokio::spawn(async move {
            run_server(&path_clone).await.unwrap();
        });

        // Give the daemon a moment to bind the socket
        sleep(Duration::from_millis(50)).await;

        // Connect Client
        let stream = UnixStream::connect(socket_path)
            .await
            .expect("Failed to connect to daemon");
        let mut client_framed = Framed::new(stream, ClientCodec::new());

        // Initiate Handshake
        client_framed
            .send(ClientMessage::Handshake { session_id: 42 })
            .await
            .unwrap();

        // Verify HandshakeAck
        let response = client_framed.next().await.unwrap().unwrap();
        assert_eq!(response, DaemonMessage::HandshakeAck);

        // Send Input to trigger DummyFrame
        client_framed
            .send(ClientMessage::Input(vec![0x0D])) // Sending a fake 'Enter' key
            .await
            .unwrap();

        // Verify DummyFrame payload
        let render_response = client_framed.next().await.unwrap().unwrap();
        match render_response {
            DaemonMessage::DummyFrame { epoch, pixels } => {
                assert_eq!(epoch, 0);
                assert_eq!(pixels.len(), 64);
                assert_eq!(pixels[0], 255);
            }
            _ => panic!("Expected DummyFrame, received something else"),
        }

        // Cleanup
        client_framed.send(ClientMessage::Detach).await.unwrap();
        daemon_handle.abort();
        let _ = fs::remove_file(socket_path);
    }
}
