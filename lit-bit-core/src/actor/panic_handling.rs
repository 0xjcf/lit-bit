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

/// Safely truncates a string slice to at most max_bytes bytes, ensuring the result ends at a UTF-8 character boundary.
/// Returns the index where the string should be truncated.
#[inline]
fn find_truncation_boundary(s: &str, max_bytes: usize) -> usize {
    let target_len = s.len().min(max_bytes);
    if target_len == s.len() {
        return target_len;
    }

    // Walk backwards from target_len until we find a char boundary
    let mut len = target_len;
    while len > 0 && !s.is_char_boundary(len) {
        len -= 1;
    }
    len
}

/// Safely pushes a string to a heapless::String, truncating if necessary to maintain capacity
/// and UTF-8 validity. Never panics.
#[cfg(not(any(feature = "std", feature = "alloc")))]
#[inline]
fn push_str_truncate<const N: usize>(dest: &mut heapless::String<N>, text: &str) {
    let target_capacity = dest.capacity();
    let start_len = dest.len();

    // Early return if no space left
    if start_len >= target_capacity {
        return;
    }

    // Process one character at a time, but look ahead to ensure we have space
    let mut chars = text.chars();
    let mut current_len = start_len;

    while let Some(ch) = chars.next() {
        let char_len = ch.len_utf8();

        // Look ahead to see if this is part of a multi-char sequence we should keep together
        let mut peek_chars = chars.clone();
        let mut peek_total = char_len;
        let mut has_next = false;

        // Try to peek at the next character (if any)
        if let Some(next_ch) = peek_chars.next() {
            has_next = true;
            peek_total += next_ch.len_utf8();
        }

        // If this character starts a sequence and we can't fit the whole sequence,
        // stop here without adding anything more
        if has_next && current_len + peek_total > target_capacity {
            break;
        }

        // If we can't fit even this single character, stop
        if current_len + char_len > target_capacity {
            break;
        }

        // Safe to add this character
        let _ = dest.push(ch);
        current_len += char_len;
    }

    debug_assert!(
        dest.len() <= target_capacity,
        "String exceeded capacity: len={}, cap={}",
        dest.len(),
        target_capacity
    );
    debug_assert!(
        dest.is_char_boundary(dest.len()),
        "String not at char boundary: len={}",
        dest.len()
    );
}

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
        push_str_truncate(&mut s, message);
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
            push_str_truncate(&mut s, actor_id.as_ref());
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
        push_str_truncate(&mut panic_msg, message);

        let actor_id_string = actor_id.map(|id| {
            let mut s = HeaplessString::<128>::new();
            push_str_truncate(&mut s, id);
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
    use core::str::from_utf8;

    #[cfg(any(feature = "std", feature = "alloc"))]
    use alloc::format;

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

    #[test]
    fn utf8_truncation_does_not_split_characters() {
        #[cfg(not(any(feature = "std", feature = "alloc")))]
        {
            let mut input = HeaplessString::<128>::new();
            // Fill with 120 'a' characters (120 bytes)
            for _ in 0..120 {
                let _ = input.push('a');
            }
            debug_assert_eq!(input.len(), 120, "Base string should be 120 bytes");

            let multi = "é€😀"; // 2 + 3 + 4 = 9 bytes
            debug_assert_eq!(
                multi.chars().next().unwrap().len_utf8(),
                2,
                "é should be 2 bytes"
            );
            debug_assert_eq!(
                multi.chars().nth(1).unwrap().len_utf8(),
                3,
                "€ should be 3 bytes"
            );

            let orig_len = input.len();
            push_str_truncate(&mut input, multi);
            debug_assert!(input.len() > orig_len, "String should have grown");
            debug_assert!(
                input.is_char_boundary(input.len()),
                "Should end at char boundary"
            );

            let error = create_controlled_failure(&input, Some("actor_é€😀"));
            match error {
                ActorError::Panic {
                    message,
                    actor_id: _,
                } => {
                    let msg = message.unwrap();
                    debug_assert!(from_utf8(msg.as_bytes()).is_ok(), "Should be valid UTF-8");
                    debug_assert!(
                        msg.is_char_boundary(msg.len()),
                        "Should end at char boundary"
                    );
                    debug_assert!(msg.len() <= 128, "Should not exceed capacity");

                    // We expect either:
                    // 1. The string to contain 120 'a's + 'é' (122 bytes total), or
                    // 2. Just 120 'a's if even 'é' wouldn't fit
                    let expected_with_e = msg.len() == 122 && msg.ends_with('é');
                    let expected_just_a = msg.len() == 120 && msg.ends_with('a');

                    // Print detailed state if assertion will fail
                    if !(expected_with_e || expected_just_a) {
                        debug_assert!(
                            false,
                            "Unexpected string state:\n\
                             - Length: {}\n\
                             - Ends with: {:?}\n\
                             - Last char len: {}\n\
                             - Capacity: {}\n\
                             - Full string: {:?}",
                            msg.len(),
                            msg.chars().last().unwrap(),
                            msg.chars().last().unwrap().len_utf8(),
                            128,
                            msg
                        );
                    }

                    assert!(
                        expected_with_e || expected_just_a,
                        "String should either end with 'é' at 122 bytes or 'a' at 120 bytes, \
                         but got len={} and ends with {:?}",
                        msg.len(),
                        msg.chars().last().unwrap()
                    );
                }
                _ => panic!("Expected Panic variant"),
            }
        }

        #[cfg(any(feature = "std", feature = "alloc"))]
        {
            let base = "a".repeat(120);
            let multi = "é€😀";
            let input = format!("{base}{multi}");
            let error = create_controlled_failure(&input, Some("actor_é€😀"));
            match error {
                ActorError::Panic {
                    message,
                    actor_id: _,
                } => {
                    let msg = message.unwrap();
                    assert!(from_utf8(msg.as_bytes()).is_ok(), "Should be valid UTF-8");
                    assert!(
                        msg.is_char_boundary(msg.len()),
                        "Should end at char boundary"
                    );
                    assert!(msg.len() <= 128, "Should not exceed capacity");

                    // Same expectations as no_std case
                    let expected_with_e = msg.len() == 122 && msg.ends_with('é');
                    let expected_just_a = msg.len() == 120 && msg.ends_with('a');
                    assert!(
                        expected_with_e || expected_just_a,
                        "String should either end with 'é' at 122 bytes or 'a' at 120 bytes, \
                         but got len={} and ends with {:?}",
                        msg.len(),
                        msg.chars().last().unwrap()
                    );
                }
                _ => panic!("Expected Panic variant"),
            }
        }
    }

    #[test]
    fn utf8_truncation_exact_boundary() {
        #[cfg(any(feature = "std", feature = "alloc"))]
        {
            let mut s = String::new();
            while s.len() < 125 {
                s.push('a');
            }
            s.push('€'); // 3 bytes, should fit exactly
            let error = create_controlled_failure(&s, None);
            match error {
                ActorError::Panic { message, .. } => {
                    let msg = message.unwrap();
                    assert!(msg.ends_with('€'));
                    assert_eq!(msg.len(), 128);
                    assert!(msg.is_char_boundary(msg.len()));
                }
                _ => panic!("Expected Panic variant"),
            }
        }
        #[cfg(not(any(feature = "std", feature = "alloc")))]
        {
            let mut s = HeaplessString::<128>::new();
            while s.len() < 125 {
                let _ = s.push('a');
            }
            let _ = s.push('€'); // 3 bytes, should fit exactly
            let error = create_controlled_failure(&s, None);
            match error {
                ActorError::Panic { message, .. } => {
                    let msg = message.unwrap();
                    assert!(msg.ends_with('€'));
                    assert_eq!(msg.len(), 128);
                    assert!(msg.is_char_boundary(msg.len()));
                }
                _ => panic!("Expected Panic variant"),
            }
        }
    }

    #[test]
    fn utf8_truncation_with_emoji() {
        #[cfg(any(feature = "std", feature = "alloc"))]
        {
            let mut s = String::new();
            while s.len() < 124 {
                s.push('b');
            }
            s.push('😀');
            let error = create_controlled_failure(&s, None);
            match error {
                ActorError::Panic { message, .. } => {
                    let msg = message.unwrap();
                    let msg_bytes = msg.as_bytes();
                    assert!(from_utf8(msg_bytes).is_ok());
                    assert!(msg.is_char_boundary(msg.len()));
                    assert!(msg.ends_with('😀') || msg.ends_with('b'));
                }
                _ => panic!("Expected Panic variant"),
            }
        }
        #[cfg(not(any(feature = "std", feature = "alloc")))]
        {
            let mut s = HeaplessString::<128>::new();
            while s.len() < 124 {
                s.push('b').unwrap();
            }
            s.push('😀').unwrap();
            let error = create_controlled_failure(&s, None);
            match error {
                ActorError::Panic { message, .. } => {
                    let msg = message.unwrap();
                    let msg_bytes = msg.as_bytes();
                    assert!(from_utf8(msg_bytes).is_ok());
                    assert!(msg.is_char_boundary(msg.len()));
                    assert!(msg.ends_with('😀') || msg.ends_with('b'));
                }
                _ => panic!("Expected Panic variant"),
            }
        }
    }

    #[cfg(feature = "async-embassy")]
    #[test]
    fn embassy_utf8_truncation_simulation() {
        #[cfg(any(feature = "std", feature = "alloc"))]
        {
            let base = "c".repeat(120);
            let multi = "é€😀";
            let input = format!("{base}{multi}");
            let error = simulate_panic_for_testing(&input);
            match error {
                ActorError::Panic { message, .. } => {
                    let msg = message.unwrap();
                    let msg_bytes = msg.as_bytes();
                    assert!(from_utf8(msg_bytes).is_ok());
                    assert!(msg.is_char_boundary(msg.len()));
                }
                _ => panic!("Expected Panic variant"),
            }
        }
        #[cfg(not(any(feature = "std", feature = "alloc")))]
        {
            let mut base = HeaplessString::<128>::new();
            for _ in 0..120 {
                base.push('c').unwrap();
            }
            let multi = "é€😀";
            let mut input = base.clone();
            input.push_str(multi).unwrap();
            let error = simulate_panic_for_testing(&input);
            match error {
                ActorError::Panic { message, .. } => {
                    let msg = message.unwrap();
                    let msg_bytes = msg.as_bytes();
                    assert!(from_utf8(msg_bytes).is_ok());
                    assert!(msg.is_char_boundary(msg.len()));
                }
                _ => panic!("Expected Panic variant"),
            }
        }
    }

    #[cfg(feature = "async-embassy")]
    #[test]
    fn embassy_utf8_truncation_with_id() {
        #[cfg(any(feature = "std", feature = "alloc"))]
        {
            let base = "d".repeat(120);
            let multi = "é€😀";
            let input = format!("{base}{multi}");
            let error = simulate_panic_with_id(&input, "actor_é€😀");
            match error {
                ActorError::Panic { message, actor_id } => {
                    let msg = message.unwrap();
                    let msg_bytes = msg.as_bytes();
                    assert!(from_utf8(msg_bytes).is_ok());
                    assert!(msg.is_char_boundary(msg.len()));
                    if let Some(actor) = actor_id {
                        let actor_bytes = actor.as_bytes();
                        assert!(from_utf8(actor_bytes).is_ok());
                        assert!(actor.is_char_boundary(actor.len()));
                    }
                }
                _ => panic!("Expected Panic variant"),
            }
        }
        #[cfg(not(any(feature = "std", feature = "alloc")))]
        {
            let mut base = HeaplessString::<128>::new();
            for _ in 0..120 {
                base.push('d').unwrap();
            }
            let multi = "é€😀";
            let mut input = base.clone();
            input.push_str(multi).unwrap();
            let error = simulate_panic_with_id(&input, "actor_é€😀");
            match error {
                ActorError::Panic { message, actor_id } => {
                    let msg = message.unwrap();
                    let msg_bytes = msg.as_bytes();
                    assert!(from_utf8(msg_bytes).is_ok());
                    assert!(msg.is_char_boundary(msg.len()));
                    if let Some(actor) = actor_id {
                        let actor_bytes = actor.as_bytes();
                        assert!(from_utf8(actor_bytes).is_ok());
                        assert!(actor.is_char_boundary(actor.len()));
                    }
                }
                _ => panic!("Expected Panic variant"),
            }
        }
    }
}
