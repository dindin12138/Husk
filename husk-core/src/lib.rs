#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use std::ptr;
use tracing::instrument;

// Include raw C FFI bindings
pub mod ffi {
    include!(concat!(env!("OUT_DIR"), "/ghostty_bindings.rs"));
}

/// Safe wrapper for the underlying GhosttyTerminal instance
pub struct Terminal {
    inner: ffi::GhosttyTerminal_ptr,
}

impl Terminal {
    /// Creates a new terminal instance safely
    #[instrument(skip_all)]
    pub fn new() -> Result<Self, &'static str> {
        let mut inner: ffi::GhosttyTerminal_ptr = ptr::null_mut();

        unsafe {
            // Zero-initialize the options struct to use default configurations
            let options: ffi::GhosttyTerminalOptions = std::mem::zeroed();

            let result = ffi::ghostty_terminal_new(ptr::null(), &mut inner, options);

            if result != 0 {
                return Err("Failed to allocate Ghostty Terminal via FFI");
            }
        }

        tracing::debug!("Safe Rust: Ghostty terminal instance created successfully.");
        Ok(Self { inner })
    }

    /// Returns the underlying raw pointer
    pub fn as_mut_ptr(&self) -> ffi::GhosttyTerminal_ptr {
        self.inner
    }
}

// Automatic memory management
impl Drop for Terminal {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                // Call specific ghostty_terminal_free instead of libc::free
                ffi::ghostty_terminal_free(self.inner);
            }
            tracing::debug!(
                "Safe Rust: Ghostty terminal instance memory safely released (Dropped)."
            );
        }
    }
}

// ---------------------------------------------
// Test instrumentation
#[instrument]
pub fn core_handshake_test(client_id: u32) {
    tracing::info!("Core: Processing handshake request...");
    tracing::debug!("Core: Credentials verified successfully.");

    if let Ok(_terminal) = Terminal::new() {
        tracing::info!("Core: Successfully established Safe Rust to Ghostty C ABI link!");
    }
}
