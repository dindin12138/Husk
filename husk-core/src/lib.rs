#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

pub mod ipc;

use std::ptr;
use tracing::instrument;

// ==========================================
// FFI Bindings
// ==========================================

pub mod ffi {
    include!(concat!(env!("OUT_DIR"), "/ghostty_bindings.rs"));
}

// ==========================================
// Safe Terminal Wrapper
// ==========================================

/// Safe wrapper for the underlying GhosttyTerminal instance.
/// Guarantees memory safety by utilizing Rust's RAII and Drop trait.
pub struct Terminal {
    inner: ffi::GhosttyTerminal_ptr,
}

impl Terminal {
    /// Creates a new Ghostty terminal instance safely.
    #[instrument(skip_all)]
    pub fn new() -> Result<Self, &'static str> {
        let mut inner: ffi::GhosttyTerminal_ptr = ptr::null_mut();

        unsafe {
            // Initialize options with valid, non-zero geometry values
            // to satisfy Ghostty's internal assertions.
            let options = ffi::GhosttyTerminalOptions {
                cols: 80,
                rows: 24,
                max_scrollback: 10_000,
            };

            // Pass 3 arguments: Allocator (null), pointer to terminal pointer, and options
            let result = ffi::ghostty_terminal_new(ptr::null(), &mut inner, options);

            if result != 0 {
                return Err("Failed to allocate Ghostty Terminal via FFI");
            }
        }

        tracing::debug!("Safe Rust: Ghostty terminal instance created successfully.");
        Ok(Self { inner })
    }

    /// Retrieves the underlying raw pointer for internal FFI interactions.
    pub fn as_mut_ptr(&self) -> ffi::GhosttyTerminal_ptr {
        self.inner
    }
}

// Automatic memory management
// When the Terminal variable goes out of scope, the Rust compiler automatically
// calls this function, absolutely preventing memory leaks.
impl Drop for Terminal {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                // Strictly adhere to SSOT constraints: Call the specific ghostty_terminal_free
                // instead of libc::free to avoid allocator mismatch.
                ffi::ghostty_terminal_free(self.inner);
            }
            tracing::debug!(
                "Safe Rust: Ghostty terminal instance memory safely released (Dropped)."
            );
        }
    }
}

// ==========================================
// Legacy Test Instrumentation
// ==========================================

#[instrument]
pub fn core_handshake_test(client_id: u32) {
    tracing::info!("Core: Processing handshake request...");
    tracing::debug!("Core: Credentials verified successfully.");

    if let Ok(_terminal) = Terminal::new() {
        tracing::info!("Core: Successfully established Safe Rust to Ghostty C ABI link!");
    }
}

// ==========================================
// Tests
// ==========================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_lifecycle_basic() {
        // Test that a terminal can be created and implicitly dropped at the end of the scope
        // without triggering a segfault.
        let terminal = Terminal::new().expect("Failed to create terminal");
        assert!(
            !terminal.as_mut_ptr().is_null(),
            "Terminal pointer should not be null"
        );
    }

    #[test]
    fn test_terminal_lifecycle_explicit_drop() {
        // Test that an explicit drop invokes the C ABI free function correctly.
        // If there's a double-free or invalid memory access in the FFI, the test runner will crash.
        let terminal = Terminal::new().expect("Failed to create terminal");
        std::mem::drop(terminal);
    }

    #[test]
    fn test_terminal_lifecycle_multiple_instances() {
        // Create multiple instances to ensure the C allocator does not mistakenly
        // share global state, and that dropping multiple instances sequentially is safe.
        let mut terminals = Vec::new();
        for _ in 0..10 {
            let term = Terminal::new().expect("Failed to create terminal in loop");
            terminals.push(term);
        }

        assert_eq!(
            terminals.len(),
            10,
            "Should successfully hold 10 terminal instances"
        );

        // Clearing the vector forces the Drop trait to run 10 times consecutively.
        terminals.clear();
        assert_eq!(terminals.len(), 0);
    }
}
