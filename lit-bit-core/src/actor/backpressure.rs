//! Platform-specific back-pressure semantics for actor mailboxes.
//!
//! This module implements different back-pressure strategies for embedded (`no_std`)
//! and standard (`std`) environments, following research recommendations:
//! - Embedded: fail-fast semantics with immediate error when queue full
//! - Std: async back-pressure via await with bounded channels

use super::{Inbox, Outbox};

/// Unified error type for message sending with platform-appropriate semantics.
#[derive(Debug, PartialEq, Eq)]
pub enum SendError<T> {
    /// Mailbox is full (embedded: immediate error when queue full)
    Full(T),
    /// Receiver has been dropped (both platforms)
    Closed(T),
}

impl<T> core::fmt::Display for SendError<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SendError::Full(_) => write!(f, "mailbox is full"),
            SendError::Closed(_) => write!(f, "receiver has been dropped"),
        }
    }
}

#[cfg(feature = "std")]
impl<T: core::fmt::Debug> std::error::Error for SendError<T> {}

/// Platform-specific back-pressure functions for `no_std` (embedded).
///
/// Uses fail-fast semantics: operations return immediately with error if mailbox is full.
/// This prevents blocking in resource-constrained embedded environments.
#[cfg(not(feature = "std"))]
pub mod embedded {
    use super::{Inbox, Outbox, SendError};

    /// Try to send a message without blocking.
    ///
    /// # Errors
    /// Returns `SendError::Full(msg)` if the mailbox is full.
    ///
    /// Note: `SendError::Closed` cannot occur with heapless queues as they
    /// cannot detect if the consumer has been dropped. This variant is only
    /// used for API consistency with the std implementation.
    pub fn try_send<T, const N: usize>(
        outbox: &mut Outbox<T, N>,
        item: T,
    ) -> Result<(), SendError<T>> {
        outbox.enqueue(item).map_err(SendError::Full)
    }

    /// Check if the mailbox is full.
    #[must_use]
    pub fn is_full<T, const N: usize>(outbox: &Outbox<T, N>) -> bool {
        !outbox.ready()
    }

    /// Get the current number of messages in the mailbox.
    #[must_use]
    pub fn len<T, const N: usize>(outbox: &Outbox<T, N>) -> usize {
        outbox.len()
    }

    /// Get the maximum capacity of the mailbox.
    #[must_use]
    pub fn capacity<T, const N: usize>(outbox: &Outbox<T, N>) -> usize {
        outbox.capacity()
    }

    /// Try to receive a message without blocking.
    ///
    /// Returns `Some(msg)` if a message is available, `None` if the mailbox is empty.
    #[must_use]
    pub fn try_recv<T, const N: usize>(inbox: &mut Inbox<T, N>) -> Option<T> {
        inbox.dequeue()
    }

    /// Check if the mailbox is empty.
    #[must_use]
    pub fn is_empty<T, const N: usize>(inbox: &Inbox<T, N>) -> bool {
        inbox.len() == 0
    }

    /// Get the current number of messages in the mailbox.
    #[must_use]
    pub fn inbox_len<T, const N: usize>(inbox: &Inbox<T, N>) -> usize {
        inbox.len()
    }
}

/// Platform-specific back-pressure functions for `std`.
///
/// Uses async back-pressure: operations await when mailbox is full, providing
/// natural flow control in async environments.
#[cfg(feature = "std")]
pub mod std_async {
    use super::{Inbox, Outbox, SendError};

    /// Send a message with async back-pressure.
    ///
    /// This function will await if the mailbox is full, providing natural back-pressure.
    ///
    /// # Errors
    /// Returns `SendError::Closed(msg)` if the receiver has been dropped.
    pub async fn send<T: Send + 'static>(outbox: &Outbox<T>, item: T) -> Result<(), SendError<T>> {
        outbox
            .send(item)
            .await
            .map_err(|err| SendError::Closed(err.0))
    }

    /// Try to send a message without blocking.
    ///
    /// # Errors
    /// Returns `SendError::Full(msg)` if the mailbox is full.
    /// Returns `SendError::Closed(msg)` if the receiver has been dropped.
    pub fn try_send<T>(outbox: &Outbox<T>, item: T) -> Result<(), SendError<T>> {
        match outbox.try_send(item) {
            Ok(()) => Ok(()),
            Err(tokio::sync::mpsc::error::TrySendError::Full(item)) => Err(SendError::Full(item)),
            Err(tokio::sync::mpsc::error::TrySendError::Closed(item)) => {
                Err(SendError::Closed(item))
            }
        }
    }

    /// Get the maximum capacity of the mailbox.
    #[must_use]
    pub fn capacity<T>(outbox: &Outbox<T>) -> usize {
        // For tokio channels, the capacity is the same as N
        // but we should still call the method for consistency
        outbox.max_capacity()
    }

    /// Receive a message with async waiting.
    ///
    /// Returns `Some(msg)` if a message is received, `None` if the sender has been dropped.
    pub async fn recv<T>(inbox: &mut Inbox<T>) -> Option<T> {
        inbox.recv().await
    }

    /// Try to receive a message without blocking.
    ///
    /// Returns `Some(msg)` if a message is available, `None` if the mailbox is empty.
    pub fn try_recv<T>(inbox: &mut Inbox<T>) -> Option<T> {
        inbox.try_recv().ok()
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    use super::*;

    use alloc::string::ToString;

    #[test]
    fn send_error_display() {
        let error = SendError::Full(42u32);
        assert_eq!(error.to_string(), "mailbox is full");

        let error = SendError::Closed(42u32);
        assert_eq!(error.to_string(), "receiver has been dropped");
    }

    #[cfg(not(feature = "std"))]
    #[test]
    fn embedded_backpressure_fail_fast() {
        let (mut outbox, _inbox): (Outbox<u32, 2>, _) = crate::static_mailbox!(TEST_QUEUE: u32, 2);

        // Fill the mailbox (heapless queues can hold N-1 items)
        assert!(embedded::try_send::<u32, 2>(&mut outbox, 1).is_ok());

        // Next send should fail immediately (fail-fast semantics)
        assert!(matches!(
            embedded::try_send::<u32, 2>(&mut outbox, 2),
            Err(SendError::Full(2))
        ));

        // Verify capacity info (heapless capacity is N-1)
        assert_eq!(embedded::capacity::<u32, 2>(&outbox), 1);
        assert!(embedded::is_full::<u32, 2>(&outbox));
    }

    #[cfg(not(feature = "std"))]
    #[test]
    fn test_capacity_fixed() {
        let (outbox, _inbox): (Outbox<u32, 4>, _) = crate::static_mailbox!(CAPACITY_TEST: u32, 4);

        // Test that our function now returns the correct capacity
        let our_capacity = embedded::capacity::<u32, 4>(&outbox);
        let actual_capacity = outbox.capacity();

        // Both should return the same value now (N-1 for heapless)
        assert_eq!(our_capacity, 3); // Our function now calls outbox.capacity()
        assert_eq!(actual_capacity, 3); // Heapless returns N-1

        // They should match now!
        assert_eq!(our_capacity, actual_capacity);
    }

    #[cfg(feature = "std")]
    #[tokio::test]
    async fn std_backpressure_try_send() {
        let (outbox, _inbox): (Outbox<u32>, _) = crate::actor::create_mailbox::<u32>(2);

        // Fill the mailbox
        assert!(std_async::try_send::<u32>(&outbox, 1).is_ok());
        assert!(std_async::try_send::<u32>(&outbox, 2).is_ok());

        // Next try_send should fail (but send() would await)
        assert!(matches!(
            std_async::try_send::<u32>(&outbox, 3),
            Err(SendError::Full(3))
        ));

        // Verify capacity info
        assert_eq!(std_async::capacity::<u32>(&outbox), 2);
    }
}
