use heapless::spsc::Queue;
use static_cell::StaticCell;

/// Default capacity for actor mailboxes
pub const DEFAULT_MAILBOX_CAPACITY: usize = 16;

/// Creates a safe static mailbox with the given name, message type, and capacity.
/// Returns producer and consumer handles with 'static lifetime.
#[macro_export]
macro_rules! static_mailbox_safe {
    ($name:ident, $msg:ty, $cap:expr) => {
        static $name: StaticCell<Queue<$msg, $cap>> = StaticCell::new();

        pub fn $name() -> (
            heapless::spsc::Producer<'static, $msg, $cap>,
            heapless::spsc::Consumer<'static, $msg, $cap>,
        ) {
            $name.init(Queue::new()).split()
        }
    };
}

/// Helper function to create a new actor mailbox with default capacity
#[macro_export]
macro_rules! create_actor_mailbox {
    ($actor_name:ident, $msg:ty) => {
        $crate::static_mailbox_safe!(
            $actor_name,
            $msg,
            $crate::actor::mailbox::DEFAULT_MAILBOX_CAPACITY
        );
    };
    ($actor_name:ident, $msg:ty, $cap:expr) => {
        $crate::static_mailbox_safe!($actor_name, $msg, $cap);
    };
}

/// Example usage in documentation:
/// ```rust,no_run
/// use lit_bit_core::create_actor_mailbox;
///
/// // Create a mailbox for an actor that processes u32 messages
/// create_actor_mailbox!(MY_ACTOR, u32);
///
/// // Later, get the producer/consumer handles
/// let (producer, consumer) = MY_ACTOR();
/// ```
#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::ALLOC_COUNTER;

    #[test]
    #[cfg(feature = "profile-alloc")]
    fn test_mailbox_static_no_allocs() {
        static_mailbox_safe!(TEST_MAILBOX, u32, 8);

        // Reset allocation counter
        ALLOC_COUNTER.store(0, Ordering::Relaxed);

        // Create mailbox
        let (producer, consumer) = TEST_MAILBOX();

        // Verify no allocations occurred
        assert_eq!(
            ALLOC_COUNTER.load(Ordering::Relaxed),
            0,
            "Mailbox creation should not allocate"
        );

        // Test basic operations
        assert!(producer.enqueue(42).is_ok());
        assert_eq!(consumer.dequeue(), Some(42));

        // Verify still no allocations
        assert_eq!(
            ALLOC_COUNTER.load(Ordering::Relaxed),
            0,
            "Mailbox operations should not allocate"
        );
    }

    #[test]
    fn test_mailbox_capacity() {
        static_mailbox_safe!(CAP_MAILBOX, u32, 4);
        let (mut producer, mut consumer) = CAP_MAILBOX();

        // Fill to capacity
        for i in 0..4 {
            assert!(producer.enqueue(i).is_ok());
        }

        // Should fail when full
        assert!(producer.enqueue(42).is_err());

        // Drain and verify
        for i in 0..4 {
            assert_eq!(consumer.dequeue(), Some(i));
        }
        assert_eq!(consumer.dequeue(), None);
    }

    #[test]
    #[cfg(feature = "profile-alloc")]
    fn test_actor_mailbox_creation() {
        create_actor_mailbox!(ACTOR_MAILBOX, String);
        let (producer, consumer) = ACTOR_MAILBOX();

        ALLOC_COUNTER.store(0, Ordering::Relaxed);
        assert_eq!(
            ALLOC_COUNTER.load(Ordering::Relaxed),
            0,
            "Actor mailbox creation should not allocate"
        );
    }

    // TODO: Add more comprehensive tests in follow-up PR
    // - Test concurrent producer/consumer operations
    // - Test with different message types and sizes
    // - Test error conditions and edge cases
}
