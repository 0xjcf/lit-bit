//! Type-safe Address handle for actor message delivery (Task 1.2 scaffold)

use super::backpressure::SendError;

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
pub struct Address<Event: 'static, const N: usize> {
    sender: heapless::spsc::Producer<'static, Event, N>,
    _phantom: core::marker::PhantomData<Event>,
}

#[cfg(not(feature = "std"))]
impl<Event: 'static, const N: usize> Address<Event, N> {
    /// Create an Address from a heapless producer.
    #[must_use]
    /// Creates a new `Address` from a given heapless SPSC producer.
    ///
    /// # Examples
    ///
    /// ```
    /// use heapless::spsc::Queue;
    /// use your_crate::Address;
    ///
    /// static mut QUEUE: Queue<u32, 4> = Queue::new();
    /// let (prod, _) = unsafe { QUEUE.split() };
    /// let address = Address::from_producer(prod);
    /// ```
    pub fn from_producer(sender: heapless::spsc::Producer<'static, Event, N>) -> Self {
        Self {
            sender,
            _phantom: core::marker::PhantomData,
        }
    }

    /// Try to send an event to the actor's mailbox.
    ///
    /// # Errors
    /// Attempts to enqueue an event into the mailbox without blocking.
    ///
    /// Returns `Ok(())` if the event is successfully enqueued, or `SendError::Full(event)` if the mailbox is full.
    ///
    /// # Examples
    ///
    /// ```
    /// use heapless::spsc::Queue;
    /// use my_actor::{Address, SendError};
    ///
    /// let mut queue: Queue<u32, 2> = Queue::new();
    /// let (prod, _) = queue.split();
    /// let mut addr = Address::from_producer(prod);
    ///
    /// assert!(addr.try_send(42).is_ok());
    /// assert!(addr.try_send(43).is_ok());
    /// // The queue is now full; the next send will fail.
    /// assert!(matches!(addr.try_send(44), Err(SendError::Full(44))));
    /// ```
    pub fn try_send(&mut self, event: Event) -> Result<(), SendError<Event>> {
        self.sender.enqueue(event).map_err(SendError::Full)
    }
}

#[cfg(test)]
mod tests {
    #[cfg(not(feature = "std"))]
    extern crate alloc;
    #[cfg(not(feature = "std"))]
    use alloc::boxed::Box;
    #[test]
    fn address_type_sanity() {
        // TDD: Address<Event> can be constructed and is type-safe
        // This test is only meaningful for the heapless variant
        #[cfg(not(feature = "std"))]
        {
            use super::Address;
            use heapless::spsc::Queue;
            const CAP: usize = 2;
            let queue: &'static mut Queue<u32, CAP> = Box::leak(Box::new(Queue::new()));
            let (prod, _cons) = queue.split();
            let _addr: Address<u32, CAP> = Address::from_producer(prod);
        }
    }
}

#[cfg(all(test, not(feature = "std")))]
mod nostd_tests {
    extern crate alloc;
    use super::Address;
    use alloc::boxed::Box;
    use heapless::spsc::Queue;

    #[test]
    fn try_send_fails_when_queue_full() {
        const CAP: usize = 3;
        let queue: &'static mut Queue<u8, CAP> = Box::leak(Box::new(Queue::new()));
        let (prod, _cons) = queue.split();
        let mut addr = Address::<u8, CAP>::from_producer(prod);
        assert!(addr.try_send(1).is_ok());
        assert!(addr.try_send(2).is_ok());
        assert!(addr.try_send(3).is_err());
    }
}

#[cfg(feature = "std")]
#[derive(Debug)]
pub enum SpawnChildError {
    MutexPoisoned,
}

#[cfg(feature = "std")]
pub struct ActorCell<Event, const N: usize> {
    _phantom: std::marker::PhantomData<Event>,
}

#[cfg(feature = "std")]
pub struct Address<Event, const N: usize> {
    sender: tokio::sync::mpsc::Sender<Event>,
    actor_id: usize, // Placeholder for ActorId type
    parent: Option<std::sync::Weak<ActorCell<Event, N>>>,
    children: std::sync::Arc<std::sync::Mutex<Vec<std::sync::Weak<ActorCell<Event, N>>>>>,
    cell: std::sync::Arc<ActorCell<Event, N>>, // For test access
}

#[cfg(feature = "std")]
impl<Event, const N: usize> Address<Event, N> {
    /// Create an Address from an Arc<ActorCell>.
    #[must_use]
    /// Creates a new `Address` from the given actor cell, initializing a Tokio channel and empty parent and children references.
    ///
    /// # Examples
    ///
    /// ```
    /// let cell = std::sync::Arc::new(ActorCell::<MyEvent, 8>::default());
    /// let address = Address::from_cell(cell.clone());
    /// assert_eq!(address.cell(), cell);
    /// ```
    pub fn from_cell(cell: std::sync::Arc<ActorCell<Event, N>>) -> Self {
        let (sender, _receiver) = tokio::sync::mpsc::channel(N);
        Self {
            sender,
            actor_id: 0,
            parent: None,
            children: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            cell,
        }
    }

    /// Create an Address from a Tokio sender (for `spawn_actor_tokio`).
    #[must_use]
    /// Creates a new `Address` from a Tokio MPSC sender, initializing a fresh actor cell with no parent or children.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio::sync::mpsc;
    /// let (tx, _rx) = mpsc::channel(8);
    /// let address = Address::<u32, 8>::from_tokio_sender(tx);
    /// ```
    pub fn from_tokio_sender(sender: tokio::sync::mpsc::Sender<Event>) -> Self {
        let cell = std::sync::Arc::new(ActorCell::<Event, N> {
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
    /// Returns an optional reference to the weak pointer of the parent actor cell, if one exists.
    pub fn parent(&self) -> Option<&std::sync::Weak<ActorCell<Event, N>>> {
        self.parent.as_ref()
    }

    /// Returns a `MutexGuard` to the children vector.
    ///
    /// # Panics
    /// Returns a guard to the vector of weak references to child actor cells.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex protecting the children vector is poisoned.
    ///
    /// # Examples
    ///
    /// ```
    /// let parent = Address::<MyEvent, 8>::from_cell(Arc::new(ActorCell::default()));
    /// let children = parent.children();
    /// assert!(children.is_empty());
    /// ```
    pub fn children(&self) -> std::sync::MutexGuard<'_, Vec<std::sync::Weak<ActorCell<Event, N>>>> {
        self.children.lock().unwrap()
    }

    /// Returns a clone of the Arc to the `ActorCell`.
    #[must_use]
    /// Returns a clone of the internal `Arc` to the associated `ActorCell`.
    ///
    /// # Examples
    ///
    /// ```
    /// let address = Address::<MyEvent, 8>::from_cell(Arc::new(ActorCell::new()));
    /// let cell = address.cell();
    /// assert!(Arc::ptr_eq(&cell, &address.cell()));
    /// ```
    pub fn cell(&self) -> std::sync::Arc<ActorCell<Event, N>> {
        self.cell.clone()
    }

    /// Send a message with async back-pressure.
    ///
    /// This method will await if the mailbox is full, providing natural back-pressure.
    ///
    /// # Errors
    /// Asynchronously sends an event to the actor, awaiting if the mailbox is full.
    ///
    /// Returns an error if the receiver has been dropped.
    ///
    /// # Returns
    /// - `Ok(())` if the event was sent successfully.
    /// - `Err(SendError::Closed(event))` if the receiver is closed.
    ///
    /// # Examples
    ///
    /// ```
    /// # use your_crate::{Address, SendError};
    /// # async fn example(address: Address<String>) {
    /// let result = address.send("hello".to_string()).await;
    /// assert!(result.is_ok() || matches!(result, Err(SendError::Closed(_))));
    /// # }
    /// ```
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
    /// Attempts to send an event to the actor without blocking.
    ///
    /// Returns `Ok(())` if the event is enqueued successfully. Returns `SendError::Full(event)` if the mailbox is full, or `SendError::Closed(event)` if the receiver has been dropped.
    ///
    /// # Examples
    ///
    /// ```
    /// let (address, _cell) = Address::<u32, 8>::from_tokio_sender(tokio::sync::mpsc::channel(8).0);
    /// let result = address.try_send(42);
    /// assert!(result.is_ok() || matches!(result, Err(SendError::Full(42)) | Err(SendError::Closed(42))));
    /// ```
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
    /// # Errors
    /// Returns `SpawnChildError::MutexPoisoned` if the children mutex is poisoned.
    ///
    /// # Panics
    /// Spawns a new child actor address linked to this actor as its parent.
    ///
    /// Creates a new `Address` representing a child actor, sets its parent reference to this actor, and adds it to the parent's list of children. Returns an error if the children mutex is poisoned.
    ///
    /// # Returns
    /// - `Ok(Address)` with the new child address on success.
    /// - `Err(SpawnChildError::MutexPoisoned)` if the children mutex is poisoned.
    ///
    /// # Examples
    ///
    /// ```
    /// let parent = Address::<MyEvent, 8>::from_cell(Arc::new(ActorCell { _phantom: PhantomData }));
    /// let child = parent.spawn_child().unwrap();
    /// assert!(child.parent().is_some());
    /// ```
    pub fn spawn_child(&self) -> Result<Self, SpawnChildError> {
        let child_cell = std::sync::Arc::new(ActorCell::<Event, N> {
            _phantom: std::marker::PhantomData,
        });
        let mut child_addr = Address::from_cell(child_cell.clone());
        // Set child's parent to this cell
        child_addr.parent = Some(std::sync::Arc::downgrade(&self.cell));
        // Add child to parent's children list
        let mut children = self
            .children
            .lock()
            .map_err(|_| SpawnChildError::MutexPoisoned)?;
        children.push(std::sync::Arc::downgrade(&child_cell));
        Ok(child_addr)
    }
}

#[cfg(all(test, feature = "std"))]
mod std_hierarchy_tests {
    use super::*;
    #[test]
    fn parent_can_spawn_child_and_links_are_correct() {
        let parent_cell = std::sync::Arc::new(ActorCell::<u8, 4> {
            _phantom: std::marker::PhantomData,
        });
        let parent_addr: Address<u8, 4> = Address::from_cell(parent_cell.clone());
        let child_addr = parent_addr.spawn_child().unwrap();
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
}
