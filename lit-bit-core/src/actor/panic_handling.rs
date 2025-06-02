//! Platform-specific panic capture utilities for Task 5.4 implementation.
//!
//! Based on research from Actix, Ractor, and Bastion panic handling patterns.
//! Provides unified panic information extraction across Tokio and Embassy runtimes.

use super::ActorError;
use core::any::Any;

// Platform-dual string support for panic information
#[cfg(any(feature = "std", feature = "alloc"))]
extern crate alloc;

#[cfg(any(feature = "std", feature = "alloc"))]
use alloc::string::String;

#[cfg(any(feature = "std", feature = "alloc"))]
use alloc::string::ToString;

#[cfg(not(any(feature = "std", feature = "alloc")))]
use heapless::String as HeaplessString;

#[cfg(not(any(feature = "std", feature = "alloc")))]
use core::str::FromStr;

/// Helper to extract a panic message from a panic payload (Box<dyn Any + Send> or &dyn Any).
#[cfg(any(feature = "std", feature = "alloc"))]
fn extract_panic_message_from_any(panic_payload: &dyn Any) -> String {
    panic_payload
        .downcast_ref::<&str>()
        .map(|s| String::from(*s))
        .or_else(|| panic_payload.downcast_ref::<String>().cloned())
        .or_else(|| {
            panic_payload
                .downcast_ref::<&'static str>()
                .map(|s| String::from(*s))
        })
        .unwrap_or_else(|| "panic occurred (payload type unknown)".to_string())
}

#[cfg(not(any(feature = "std", feature = "alloc")))]
fn extract_panic_message_from_any(panic_payload: &dyn Any) -> Option<HeaplessString<128>> {
    panic_payload
        .downcast_ref::<&str>()
        .and_then(|s| HeaplessString::<128>::from_str(s).ok())
        .or_else(|| panic_payload.downcast_ref::<HeaplessString<128>>().cloned())
        .or_else(|| {
            panic_payload
                .downcast_ref::<&'static str>()
                .and_then(|s| HeaplessString::<128>::from_str(s).ok())
        })
}

/// Tokio-specific panic capture utilities using JoinError introspection.
///
/// This function extracts panic information from Tokio's JoinError, following
/// the pattern used by production actor frameworks like Ractor and Bastion.
#[cfg(feature = "async-tokio")]
pub fn capture_panic_info(join_error: tokio::task::JoinError) -> ActorError {
    if join_error.is_panic() {
        // Extract panic payload and try to downcast to common string types
        let panic_any = join_error.into_panic();
        let panic_message = extract_panic_message_from_any(panic_any.as_ref());

        ActorError::Panic {
            message: Some(panic_message),
            actor_id: None, // Can be filled by caller
        }
    } else {
        // JoinError but not a panic (task was cancelled or aborted)
        ActorError::ShutdownFailure
    }
}

/// Extracts panic information from a panic payload (for use with catch_unwind).
///
/// This function processes the `Box<dyn Any + Send>` payload from `std::panic::catch_unwind()`
/// and attempts to extract a meaningful panic message for supervision decisions.
#[cfg(feature = "async-tokio")]
pub fn capture_panic_info_from_payload(
    panic_payload: &Box<dyn std::any::Any + Send>,
) -> ActorError {
    let panic_message = extract_panic_message_from_any(panic_payload.as_ref());

    ActorError::Panic {
        message: Some(panic_message),
        actor_id: None, // Can be filled by caller
    }
}

/// Enhanced panic info capture from payload with actor ID context.
///
/// This variant allows the caller to provide actor identification context
/// for better supervision decision making.
#[cfg(feature = "async-tokio")]
pub fn capture_panic_info_from_payload_with_id(
    panic_payload: &Box<dyn std::any::Any + Send>,
    actor_id: impl Into<String>,
) -> ActorError {
    let mut error = capture_panic_info_from_payload(panic_payload);

    if let ActorError::Panic {
        actor_id: ref mut id,
        ..
    } = error
    {
        *id = Some(actor_id.into());
    }

    error
}

/// Enhanced panic info capture with actor ID context.
///
/// This variant allows the caller to provide actor identification context
/// for better supervision decision making.
#[cfg(feature = "async-tokio")]
pub fn capture_panic_info_with_id(
    join_error: tokio::task::JoinError,
    actor_id: impl Into<String>,
) -> ActorError {
    let mut error = capture_panic_info(join_error);

    if let ActorError::Panic {
        actor_id: ref mut id,
        ..
    } = error
    {
        *id = Some(actor_id.into());
    }

    error
}

/// Embassy-specific panic simulation for testing and controlled error scenarios.
///
/// Since Embassy runs on no_std and cannot use unwinding panic recovery,
/// this function creates panic-like errors for testing supervision logic.
#[cfg(feature = "async-embassy")]
pub fn simulate_panic_for_testing(message: &str) -> ActorError {
    #[cfg(any(feature = "std", feature = "alloc"))]
    let panic_message = message.to_string();

    #[cfg(not(any(feature = "std", feature = "alloc")))]
    let panic_message = {
        let mut s = HeaplessString::<128>::new();
        // Truncate at a valid UTF-8 boundary so we don't panic
        let mut end = 0;
        for (idx, _) in message.char_indices() {
            if idx <= 127 {
                end = idx;
            } else {
                break;
            }
        }
        if end == 0 && !message.is_empty() {
            // The first char is >127 bytes in, so just take the first char
            end = message.len().min(127);
        } else {
            // Include the last char if it fits
            end = (end + message[end..].chars().next().map_or(0, |c| c.len_utf8()))
                .min(message.len())
                .min(128);
        }
        let _ = s.push_str(&message[..end]);
        s
    };

    ActorError::Panic {
        message: Some(panic_message),
        actor_id: None,
    }
}

/// Embassy-specific panic simulation with actor ID context.
#[cfg(feature = "async-embassy")]
pub fn simulate_panic_with_id(message: &str, actor_id: impl AsRef<str>) -> ActorError {
    let mut error = simulate_panic_for_testing(message);

    if let ActorError::Panic {
        actor_id: ref mut id,
        ..
    } = error
    {
        #[cfg(any(feature = "std", feature = "alloc"))]
        {
            *id = Some(actor_id.as_ref().to_string());
        }

        #[cfg(not(any(feature = "std", feature = "alloc")))]
        {
            let mut s = HeaplessString::<128>::new();
            let actor_str = actor_id.as_ref();
            let mut end = 0;
            for (idx, _) in actor_str.char_indices() {
                if idx <= 127 {
                    end = idx;
                } else {
                    break;
                }
            }
            if end == 0 && !actor_str.is_empty() {
                end = actor_str.len().min(127);
            } else {
                end = (end + actor_str[end..].chars().next().map_or(0, |c| c.len_utf8()))
                    .min(actor_str.len())
                    .min(128);
            }
            let _ = s.push_str(&actor_str[..end]);
            *id = Some(s);
        }
    }

    error
}

/// Platform-agnostic error creation for controlled failure scenarios.
///
/// This function can be used by actors to signal controlled failures that
/// should be treated like panics by the supervision system. Useful for
/// implementing graceful degradation or testing supervision logic.
pub fn create_controlled_failure(message: &str, actor_id: Option<&str>) -> ActorError {
    #[cfg(any(feature = "std", feature = "alloc"))]
    {
        ActorError::Panic {
            message: Some(message.to_string()),
            actor_id: actor_id.map(|id| id.to_string()),
        }
    }

    #[cfg(not(any(feature = "std", feature = "alloc")))]
    {
        let mut panic_msg = HeaplessString::<128>::new();
        let mut end = 0;
        for (idx, _) in message.char_indices() {
            if idx <= 127 {
                end = idx;
            } else {
                break;
            }
        }
        if end == 0 && !message.is_empty() {
            end = message.len().min(127);
        } else {
            end = (end + message[end..].chars().next().map_or(0, |c| c.len_utf8()))
                .min(message.len())
                .min(128);
        }
        let _ = panic_msg.push_str(&message[..end]);

        let actor_id_string = actor_id.map(|id| {
            let mut s = HeaplessString::<128>::new();
            let mut end = 0;
            for (idx, _) in id.char_indices() {
                if idx <= 127 {
                    end = idx;
                } else {
                    break;
                }
            }
            if end == 0 && !id.is_empty() {
                end = id.len().min(127);
            } else {
                end = (end + id[end..].chars().next().map_or(0, |c| c.len_utf8()))
                    .min(id.len())
                    .min(128);
            }
            let _ = s.push_str(&id[..end]);
            s
        });

        ActorError::Panic {
            message: Some(panic_msg),
            actor_id: actor_id_string,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_controlled_failure_works() {
        let error = create_controlled_failure("test failure", Some("test_actor"));

        match error {
            ActorError::Panic { message, actor_id } => {
                assert!(message.is_some());
                assert!(actor_id.is_some());
                assert!(message.unwrap().contains("test failure"));
                assert!(actor_id.unwrap().contains("test_actor"));
            }
            _ => panic!("Expected Panic variant"),
        }
    }

    #[test]
    fn create_controlled_failure_without_actor_id() {
        let error = create_controlled_failure("test failure", None);

        match error {
            ActorError::Panic { message, actor_id } => {
                assert!(message.is_some());
                assert!(actor_id.is_none());
            }
            _ => panic!("Expected Panic variant"),
        }
    }

    #[cfg(feature = "async-embassy")]
    #[test]
    fn embassy_panic_simulation_works() {
        let error = simulate_panic_for_testing("embassy test panic");

        match error {
            ActorError::Panic {
                message,
                actor_id: _,
            } => {
                assert!(message.is_some());
                assert!(message.unwrap().contains("embassy test panic"));
            }
            _ => panic!("Expected Panic variant"),
        }
    }

    #[cfg(feature = "async-embassy")]
    #[test]
    fn embassy_panic_simulation_with_id_works() {
        let error = simulate_panic_with_id("embassy test panic", "embassy_actor");

        match error {
            ActorError::Panic { message, actor_id } => {
                assert!(message.is_some());
                assert!(actor_id.is_some());
                assert!(message.unwrap().contains("embassy test panic"));
                assert!(actor_id.unwrap().contains("embassy_actor"));
            }
            _ => panic!("Expected Panic variant"),
        }
    }
}
