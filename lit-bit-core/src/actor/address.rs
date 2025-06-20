//! Type-safe Address handle for actor message delivery (Task 1.2 scaffold)

use super::backpressure::SendError;

// Embassy-specific Address implementation
#[cfg(feature = "async-embassy")]
pub struct Address<Event: 'static, const N: usize = 32> {
    sender: embassy_sync::channel::Sender<
        'static,
        embassy_sync::blocking_mutex::raw::NoopRawMutex,
        Event,
        N,
    >,
}

#[cfg(feature = "async-embassy")]
impl<Event: 'static, const N: usize> Address<Event, N> {
    /// Create an Address from an Embassy channel sender.
    #[must_use]
    pub fn from_embassy_sender(
        sender: embassy_sync::channel::Sender<
            'static,
            embassy_sync::blocking_mutex::raw::NoopRawMutex,
            Event,
            N,
        >,
    ) -> Self {
        Self { sender }
    }

    /// Send a message with async back-pressure.
    ///
    /// This method will await if the channel is full, providing natural back-pressure.
    /// In Embassy, this integrates with the cooperative scheduler to yield when blocked.
    ///
    /// ## Embassy Channel Semantics
    ///
    /// Embassy channels are designed to be **infallible** for embedded use cases. Unlike Tokio
    /// channels, Embassy channels cannot be "closed" - they are expected to live for the
    /// program's lifetime. This method will **always return `Ok(())`** under normal conditions.
    ///
    /// ## Behavioral Differences from Tokio
    ///
    /// - **Embassy**: Never returns `Err` - channels cannot be closed, receivers dropping
    ///   causes potential deadlock (sender blocks forever) rather than immediate error
    /// - **Tokio**: Returns `Err(SendError::Closed(_))` when receiver is dropped
    ///
    /// ## When Deadlock Can Occur
    ///
    /// If the receiving task is dropped or stops consuming messages, this method will:
    /// 1. Fill the channel buffer (up to capacity `N`)
    /// 2. Block indefinitely waiting for buffer space that will never become available
    ///
    /// **Prevention**: Ensure receiving tasks run for the actor's intended lifetime, typically
    /// until system reset in embedded applications.
    ///
    /// # Errors
    ///
    /// Currently never returns an error in Embassy 0.6. The `Result` type is provided for:
    /// - **API consistency** with Tokio backend
    /// - **Future compatibility** if Embassy adds channel closure semantics
    /// - **Custom error detection** if application-level "actor alive" checks are added
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// // Send a message - will always succeed in Embassy unless system is broken
    /// match address.send(MyMessage::DoWork).await {
    ///     Ok(()) => println!("Message sent successfully"),
    ///     Err(_) => unreachable!("Embassy channels never fail in 0.6"),
    /// }
    ///
    /// // More typical Embassy usage - assume infallible
    /// address.send(MyMessage::DoWork).await.expect("Embassy send should never fail");
    /// ```
    pub async fn send(&self, event: Event) -> Result<(), SendError<Event>> {
        // Embassy channels are infallible by design - this will never fail
        // unless we add custom "actor alive" checks in the future
        self.sender.send(event).await;
        Ok(())
    }

    /// Try to send a message without blocking.
    ///
    /// This is useful when you want to avoid blocking the current task and handle
    /// backpressure explicitly.
    ///
    /// ## Embassy Channel Behavior
    ///
    /// Embassy channels only have one failure mode: `SendError::Full` when the buffer
    /// is at capacity. **There is no `SendError::Closed` variant** because Embassy
    /// channels cannot be closed - they are designed for static, long-lived usage.
    ///
    /// ## Important: No "Closed" Detection
    ///
    /// If the receiving task has stopped consuming messages (or been dropped), this method:
    /// - **Will succeed** as long as the buffer has space (placing messages nobody will read)
    /// - **Will return `Full`** once the buffer fills up (NOT a "Closed" error)
    /// - **Cannot distinguish** between "slow consumer" and "no consumer"
    ///
    /// This means message loss is possible if you drop messages on `Full` errors without
    /// knowing whether a consumer is still active.
    ///
    /// # Errors
    /// Returns `SendError::Full(msg)` if the channel buffer is at capacity.
    /// **Never returns `SendError::Closed`** in Embassy - that variant is unused.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// match address.try_send(message) {
    ///     Ok(()) => println!("Message queued successfully"),
    ///     Err(SendError::Full(msg)) => {
    ///         println!("Channel buffer full - might be slow/missing consumer");
    ///         // Handle backpressure (e.g., retry later, drop message, etc.)
    ///     }
    ///     Err(SendError::Closed(_)) => {
    ///         unreachable!("Embassy channels never close in 0.6");
    ///     }
    /// }
    /// ```
    pub fn try_send(&self, event: Event) -> Result<(), SendError<Event>> {
        match self.sender.try_send(event) {
            Ok(()) => Ok(()),
            Err(embassy_sync::channel::TrySendError::Full(event)) => Err(SendError::Full(event)),
        }
    }
}

#[cfg(feature = "async-embassy")]
impl<Event: 'static, const N: usize> Clone for Address<Event, N> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender,
        }
    }
}

// Tokio-specific Address implementation (only when Embassy is not enabled)
#[cfg(all(feature = "async-tokio", not(feature = "async-embassy")))]
#[derive(Debug)]
pub enum SpawnChildError {
    MutexPoisoned,
}

#[cfg(all(feature = "async-tokio", not(feature = "async-embassy")))]
pub struct ActorCell<Event> {
    _phantom: std::marker::PhantomData<Event>,
}

#[cfg(all(feature = "async-tokio", not(feature = "async-embassy")))]
pub struct Address<Event> {
    sender: tokio::sync::mpsc::Sender<Event>,
    actor_id: usize, // Placeholder for ActorId type
    parent: Option<std::sync::Weak<ActorCell<Event>>>,
    children: std::sync::Arc<std::sync::Mutex<Vec<std::sync::Weak<ActorCell<Event>>>>>,
    cell: std::sync::Arc<ActorCell<Event>>, // For test access
}

#[cfg(all(feature = "async-tokio", not(feature = "async-embassy")))]
impl<Event> Address<Event> {
    /// Create an Address from an Arc<ActorCell>.
    ///
    /// Returns both the Address and the receiver end of the channel.
    /// The caller is responsible for handling the receiver (e.g., passing it to an actor task).
    ///
    /// # Returns
    /// A tuple of (Address, Receiver) where the receiver must be used to avoid channel closure.
    #[must_use]
    pub fn from_cell(
        cell: std::sync::Arc<ActorCell<Event>>,
        capacity: usize,
    ) -> (Self, tokio::sync::mpsc::Receiver<Event>) {
        let (sender, receiver) = tokio::sync::mpsc::channel(capacity);
        let address = Self {
            sender,
            actor_id: 0,
            parent: None,
            children: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            cell,
        };
        (address, receiver)
    }

    /// Create an Address from a Tokio sender (for `spawn_actor_tokio`).
    #[must_use]
    pub fn from_tokio_sender(sender: tokio::sync::mpsc::Sender<Event>) -> Self {
        let cell = std::sync::Arc::new(ActorCell::<Event> {
            _phantom: std::marker::PhantomData,
        });
        Self {
            sender,
            actor_id: 0,
            parent: None,
            children: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            cell,
        }
    }

    /// Returns a reference to the parent Weak pointer, if any.
    #[must_use]
    pub fn parent(&self) -> Option<&std::sync::Weak<ActorCell<Event>>> {
        self.parent.as_ref()
    }

    /// Returns a `MutexGuard` to the children vector.
    ///
    /// # Panics
    /// Panics if the mutex is poisoned.
    pub fn children(&self) -> std::sync::MutexGuard<'_, Vec<std::sync::Weak<ActorCell<Event>>>> {
        self.children.lock().unwrap()
    }

    /// Returns a clone of the Arc to the `ActorCell`.
    #[must_use]
    pub fn cell(&self) -> std::sync::Arc<ActorCell<Event>> {
        self.cell.clone()
    }

    /// Send a message with async back-pressure.
    ///
    /// This method will await if the mailbox is full, providing natural back-pressure.
    ///
    /// # Errors
    /// Returns `SendError::Closed(msg)` if the receiver has been dropped.
    pub async fn send(&self, event: Event) -> Result<(), SendError<Event>> {
        self.sender
            .send(event)
            .await
            .map_err(|err| SendError::Closed(err.0))
    }

    /// Try to send a message without blocking.
    ///
    /// # Errors
    /// Returns `SendError::Full(msg)` if the mailbox is full.
    /// Returns `SendError::Closed(msg)` if the receiver has been dropped.
    pub fn try_send(&self, event: Event) -> Result<(), SendError<Event>> {
        match self.sender.try_send(event) {
            Ok(()) => Ok(()),
            Err(tokio::sync::mpsc::error::TrySendError::Full(event)) => Err(SendError::Full(event)),
            Err(tokio::sync::mpsc::error::TrySendError::Closed(event)) => {
                Err(SendError::Closed(event))
            }
        }
    }

    /// Spawns a child actor, linking parent and child.
    ///
    /// Returns both the child Address and the receiver end of the channel.
    /// The caller is responsible for handling the receiver (e.g., passing it to an actor task).
    ///
    /// # Errors
    /// Returns `SpawnChildError::MutexPoisoned` if the children mutex is poisoned.
    ///
    /// # Returns
    /// A tuple of (`child_address`, receiver) where the receiver must be used to avoid channel closure.
    pub fn spawn_child(
        &self,
        capacity: usize,
    ) -> Result<(Self, tokio::sync::mpsc::Receiver<Event>), SpawnChildError> {
        let child_cell = std::sync::Arc::new(ActorCell::<Event> {
            _phantom: std::marker::PhantomData,
        });
        let (mut child_addr, receiver) = Address::from_cell(child_cell.clone(), capacity);
        // Set child's parent to this cell
        child_addr.parent = Some(std::sync::Arc::downgrade(&self.cell));
        // Add child to parent's children list
        let mut children = self
            .children
            .lock()
            .map_err(|_| SpawnChildError::MutexPoisoned)?;
        children.push(std::sync::Arc::downgrade(&child_cell));
        Ok((child_addr, receiver))
    }
}

// No-std Address implementation (existing heapless-based)
#[cfg(all(not(feature = "async-tokio"), not(feature = "async-embassy")))]
pub struct Address<Event: 'static, const N: usize> {
    sender: heapless::spsc::Producer<'static, Event, N>,
    _phantom: core::marker::PhantomData<Event>,
}

#[cfg(all(not(feature = "async-tokio"), not(feature = "async-embassy")))]
impl<Event: 'static, const N: usize> Address<Event, N> {
    /// Create an Address from a heapless producer.
    #[must_use]
    pub fn from_producer(sender: heapless::spsc::Producer<'static, Event, N>) -> Self {
        Self {
            sender,
            _phantom: core::marker::PhantomData,
        }
    }

    /// Try to send an event to the actor's mailbox.
    ///
    /// # Errors
    /// Returns `SendError::Full(event)` if the mailbox is full.
    pub fn try_send(&mut self, event: Event) -> Result<(), SendError<Event>> {
        self.sender.enqueue(event).map_err(SendError::Full)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn address_type_sanity() {
        // TDD: Address<Event> can be constructed and is type-safe
        // This test is only meaningful for the heapless variant
        #[cfg(all(not(feature = "async-tokio"), not(feature = "async-embassy")))]
        {
            use super::Address;
            const CAP: usize = 2;
            let (prod, _cons) = crate::static_mailbox!(TEST_QUEUE: u32, CAP);
            let _addr: Address<u32, CAP> = Address::from_producer(prod);
        }
    }
}

#[cfg(all(test, not(feature = "async-tokio"), not(feature = "async-embassy")))]
mod nostd_tests {
    use super::Address;

    #[test]
    fn try_send_fails_when_queue_full() {
        const CAP: usize = 3;
        let (prod, _cons) = crate::static_mailbox!(FULL_QUEUE_TEST: u8, CAP);
        let mut addr = Address::<u8, CAP>::from_producer(prod);
        assert!(addr.try_send(1).is_ok());
        assert!(addr.try_send(2).is_ok());
        assert!(addr.try_send(3).is_err());
    }
}

#[cfg(all(test, feature = "async-embassy"))]
mod embassy_tests {
    use super::*;

    #[test]
    fn embassy_address_compiles() {
        // Test that Embassy Address compiles correctly
        // Actual runtime testing would require an Embassy executor

        fn test_address_signature() {
            fn _test(
                sender: embassy_sync::channel::Sender<
                    'static,
                    embassy_sync::blocking_mutex::raw::NoopRawMutex,
                    u32,
                    32,
                >,
            ) {
                let _address = Address::from_embassy_sender(sender);
            }
        }

        test_address_signature();
    }
}

#[cfg(all(test, feature = "async-tokio", not(feature = "async-embassy")))]
mod std_hierarchy_tests {
    use super::*;
    #[test]
    fn parent_can_spawn_child_and_links_are_correct() {
        let parent_cell = std::sync::Arc::new(ActorCell::<u8> {
            _phantom: std::marker::PhantomData,
        });
        let (parent_addr, _parent_receiver): (Address<u8>, _) =
            Address::from_cell(parent_cell.clone(), 4);
        let (child_addr, _child_receiver) = parent_addr.spawn_child(4).unwrap();
        // Validate child -> parent link
        let parent_ref = child_addr.parent().unwrap().upgrade().unwrap();
        assert!(std::sync::Arc::ptr_eq(&parent_ref, &parent_cell));
        // Validate parent -> child link
        let children = parent_addr.children();
        assert!(children.iter().any(|c| {
            c.upgrade()
                .is_some_and(|child_cell| std::sync::Arc::ptr_eq(&child_cell, &child_addr.cell()))
        }));
    }

    #[tokio::test]
    async fn from_cell_creates_working_address() {
        let cell = std::sync::Arc::new(ActorCell::<u32> {
            _phantom: std::marker::PhantomData,
        });
        let (addr, mut receiver) = Address::from_cell(cell, 4);

        // Test that we can send messages successfully
        assert!(addr.try_send(42).is_ok());
        assert!(addr.try_send(100).is_ok());

        // Test that we can receive the messages
        assert_eq!(receiver.recv().await, Some(42));
        assert_eq!(receiver.recv().await, Some(100));

        // Test async send as well
        assert!(addr.send(200).await.is_ok());
        assert_eq!(receiver.recv().await, Some(200));
    }

    #[tokio::test]
    async fn spawn_child_creates_working_address() {
        let parent_cell = std::sync::Arc::new(ActorCell::<u32> {
            _phantom: std::marker::PhantomData,
        });
        let (parent_addr, _parent_receiver) = Address::from_cell(parent_cell, 4);
        let (child_addr, mut child_receiver) = parent_addr.spawn_child(4).unwrap();

        // Test that we can send messages to the child successfully
        assert!(child_addr.try_send(123).is_ok());
        assert_eq!(child_receiver.recv().await, Some(123));

        // Test async send to child
        assert!(child_addr.send(456).await.is_ok());
        assert_eq!(child_receiver.recv().await, Some(456));
    }
}
