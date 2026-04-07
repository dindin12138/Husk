use tracing::instrument;

#[instrument]
pub fn core_handshake_test(client_id: u32) {
    tracing::info!("Core: A handshake request is currently being processed...");
    tracing::debug!("Core: The verification of the certificate was successful.");
}
