//! # Understanding Actor Model & Mailbox Patterns Through a Coffee Shop
//!
//! This example demonstrates core concepts of the actor model and mailbox patterns using a relatable
//! coffee shop analogy. It shows how complex systems can be modeled using actors that communicate
//! through message passing.
//!
//! ## 🎭 The Actor Model Explained
//!
//! In the actor model:
//! - Actors are independent units that can:
//!   1. Receive and process messages
//!   2. Make local decisions
//!   3. Create other actors
//!   4. Send messages to other actors
//!
//! In our coffee shop:
//! - **Baristas** and **Cashiers** are actors
//! - Each has their own state (like being on break or having an open register)
//! - They communicate only through messages (orders and payments)
//! - They never directly access each other's state
//!
//! ## 📬 Mailbox Pattern
//!
//! The mailbox pattern is how actors communicate safely:
//!
//! ```text
//! Customer -> [Order Queue] -> Barista
//!          -> [Payment Queue] -> Cashier
//! ```
//!
//! Real-world parallels:
//! - Order tickets on a spike = Message queue
//! - Full spike = Backpressure
//! - Taking one ticket at a time = Message processing
//!
//! ## 🏗️ Implementation Patterns
//!
//! This example shows two ways to create mailboxes:
//!
//! 1. **`static_mailbox!` Pattern** (Recommended)
//!    ```rust
//!    let (tx, rx) = static_mailbox!(MAILBOX: Message, 16);
//!    ```
//!    - Used for the barista's order queue
//!    - Simple and ergonomic
//!    - Compile-time memory allocation
//!
//! 2. **`create_mailbox` Pattern** (Advanced)
//!    ```rust
//!    static QUEUE: StaticCell<Queue<Message, 32>> = StaticCell::new();
//!    let (tx, rx) = create_mailbox(&QUEUE);
//!    ```
//!    - Used for the cashier's payment queue
//!    - More control over memory placement
//!    - Useful for embedded systems
//!
//! ## 🔄 State Management
//!
//! Each actor manages its own state:
//!
//! ```rust
//! struct Barista {
//!     drinks_made: u32,
//!     is_on_break: bool,
//!     needs_restock: bool,
//! }
//! ```
//!
//! State changes only happen:
//! 1. In response to messages
//! 2. Based on internal logic
//! 3. Without external interference
//!
//! ## 📨 Message Types
//!
//! Messages are strongly typed and represent specific commands:
//!
//! ```rust
//! enum BaristaMessage {
//!     MakeDrink { drink: DrinkType, extra_shot: bool },
//!     TakeBreak,
//!     RestockSupplies,
//! }
//! ```
//!
//! ## 🛡️ Error Handling & Backpressure
//!
//! The system handles various failure modes:
//! - Full queues (backpressure)
//! - Unavailable actors (on break)
//! - Resource limitations (needs restock)
//!
//! ## 🔍 Key Concepts Demonstrated
//!
//! 1. **Message Passing**
//!    - Actors communicate only through messages
//!    - No shared state
//!    - Type-safe message definitions
//!
//! 2. **State Isolation**
//!    - Each actor owns its state
//!    - State changes are localized
//!    - No external state modification
//!
//! 3. **Concurrency Model**
//!    - Lock-free message passing
//!    - Independent actor processing
//!    - Natural concurrent design
//!
//! ## 🚀 Running the Example
//!
//! ```bash
//! # Run in std mode (with visual output)
//! cargo run --example coffee_shop
//!
//! # Run in no_std mode (embedded systems)
//! cargo run --example coffee_shop --no-default-features --features panic-halt
//! ```
//!
//! ## 📚 Further Reading
//!
//! - [Actor Model](https://en.wikipedia.org/wiki/Actor_model)
//! - [Message Passing](https://en.wikipedia.org/wiki/Message_passing)
//! - [Concurrent Computing](https://en.wikipedia.org/wiki/Concurrent_computing)

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), no_main)]

use heapless::spsc::Queue;
use lit_bit_core::{Actor, create_mailbox, static_mailbox};
use static_cell::StaticCell;

#[cfg(not(feature = "std"))]
use lit_bit_core::SendError;

#[cfg(not(feature = "std"))]
extern crate panic_halt;

/// Represents different types of drinks a barista can make.
///
/// This enum demonstrates how to model domain-specific choices
/// in a type-safe way. Each variant represents a valid drink option,
/// making it impossible to order an invalid drink.
#[derive(Debug)]
enum DrinkType {
    /// A single shot of espresso
    Espresso,
    /// Espresso with steamed milk
    Latte,
    /// Espresso with steamed milk and foam
    Cappuccino,
    /// Espresso with hot water
    Americano,
}

/// Messages that can be sent to the barista actor.
///
/// This enum demonstrates the command pattern in actor systems.
/// Each variant represents a distinct operation that the barista
/// can perform, with any necessary parameters included in the variant.
#[derive(Debug)]
enum BaristaMessage {
    /// Request to make a new drink
    ///
    /// # Parameters
    /// - `drink`: The type of drink to make
    /// - `extra_shot`: Whether to add an extra shot of espresso
    /// - `order_number`: Unique identifier for this order
    MakeDrink {
        drink: DrinkType,
        extra_shot: bool,
        order_number: u32,
    },
    /// Signal the barista to take a break
    TakeBreak,
    /// Request to restock supplies
    RestockSupplies,
}

/// Messages that can be sent to the cashier actor.
///
/// This demonstrates how different actors can have different
/// message types appropriate to their responsibilities.
#[derive(Debug)]
enum CashierMessage {
    /// Process payment for a new order
    NewOrder {
        /// Amount to charge
        amount: f32,
        /// Order identifier
        order_number: u32,
    },
    /// Process a refund
    Refund {
        /// Amount to refund
        amount: f32,
        /// Order identifier
        order_number: u32,
    },
    /// Close the register for the day
    CloseRegister,
}

/// A barista actor that makes drinks and manages their own state.
///
/// This struct demonstrates key actor principles:
/// - Encapsulated state
/// - Message-driven behavior
/// - Independent decision making
struct Barista {
    /// Number of drinks made in this shift
    drinks_made: u32,
    /// Whether the barista is currently on break
    is_on_break: bool,
    /// Whether supplies need restocking
    needs_restock: bool,
}

impl Barista {
    /// Creates a new barista actor in a ready state.
    ///
    /// This demonstrates actor initialization:
    /// - Starting with a clean state
    /// - No initial messages
    /// - Ready to process work
    fn new() -> Self {
        Self {
            drinks_made: 0,
            is_on_break: false,
            needs_restock: false,
        }
    }

    /// Simulates drink preparation and manages resources.
    ///
    /// This method shows how actors:
    /// - Make local decisions (when to restock)
    /// - Manage their own state (drink counter)
    /// - Have domain-specific logic (preparation times)
    ///
    /// # Parameters
    /// - `drink`: The type of drink to prepare
    /// - `extra_shot`: Whether to add an extra shot
    ///
    /// # Returns
    /// The time in seconds needed to prepare the drink
    fn make_drink(&mut self, drink: &DrinkType, extra_shot: bool) -> u32 {
        // Track drinks made and determine if restocking is needed
        self.drinks_made += 1;
        if self.drinks_made % 10 == 0 {
            self.needs_restock = true;
        }

        // Simulate preparation times for different drinks
        match drink {
            DrinkType::Espresso => {
                if extra_shot {
                    45
                } else {
                    30
                }
            }
            DrinkType::Latte => {
                if extra_shot {
                    180
                } else {
                    150
                }
            }
            DrinkType::Cappuccino => {
                if extra_shot {
                    160
                } else {
                    140
                }
            }
            DrinkType::Americano => {
                if extra_shot {
                    90
                } else {
                    75
                }
            }
        }
    }
}

/// Implementation of the Actor trait for Barista.
///
/// This demonstrates how actors:
/// 1. Define their message type
/// 2. Process messages asynchronously
/// 3. Maintain state consistency
/// 4. Handle failure conditions
impl Actor for Barista {
    /// The type of messages this actor can receive
    type Message = BaristaMessage;

    /// The future returned by message handling
    type Future<'a>
        = core::future::Ready<()>
    where
        Self: 'a;

    /// Processes incoming messages and updates actor state.
    ///
    /// This method demonstrates:
    /// - Message pattern matching
    /// - State-dependent behavior
    /// - Error conditions
    /// - Async processing
    fn handle(&mut self, msg: Self::Message) -> Self::Future<'_> {
        match msg {
            BaristaMessage::MakeDrink {
                drink,
                extra_shot,
                order_number,
            } => {
                // Check actor state before processing
                if self.is_on_break {
                    #[cfg(feature = "std")]
                    println!(
                        "🚫 Sorry, barista is on break! Order #{} will have to wait.",
                        order_number
                    );
                    return core::future::ready(());
                }

                if self.needs_restock {
                    #[cfg(feature = "std")]
                    println!("⚠️ Need to restock supplies before making more drinks!");
                    return core::future::ready(());
                }

                // Process the drink order
                let prep_time = self.make_drink(&drink, extra_shot);
                #[cfg(feature = "std")]
                println!(
                    "☕ Order #{}: Making {:?}{} (takes {} seconds)",
                    order_number,
                    drink,
                    if extra_shot { " with extra shot" } else { "" },
                    prep_time / 60
                );
            }
            BaristaMessage::TakeBreak => {
                self.is_on_break = true;
                #[cfg(feature = "std")]
                println!("🌟 Barista is taking a well-deserved break!");
            }
            BaristaMessage::RestockSupplies => {
                self.needs_restock = false;
                #[cfg(feature = "std")]
                println!("📦 Restocking supplies...");
            }
        }
        core::future::ready(())
    }
}

/// A cashier actor that handles financial transactions.
///
/// This demonstrates:
/// - Different actor types in the same system
/// - Specialized message types per actor
/// - Independent state management
struct Cashier {
    /// Total sales for the day
    total_sales: f32,
    /// Whether the register is open for transactions
    register_open: bool,
}

impl Cashier {
    /// Creates a new cashier actor ready for business.
    fn new() -> Self {
        Self {
            total_sales: 0.0,
            register_open: true,
        }
    }
}

/// Implementation of the Actor trait for Cashier.
///
/// This shows how different actors:
/// - Handle different message types
/// - Maintain separate states
/// - Have their own processing rules
impl Actor for Cashier {
    type Message = CashierMessage;
    type Future<'a>
        = core::future::Ready<()>
    where
        Self: 'a;

    fn handle(&mut self, msg: Self::Message) -> Self::Future<'_> {
        match msg {
            CashierMessage::NewOrder {
                amount,
                order_number,
            } => {
                if !self.register_open {
                    #[cfg(feature = "std")]
                    println!(
                        "🚫 Register is closed! Can't process order #{}",
                        order_number
                    );
                    return core::future::ready(());
                }

                self.total_sales += amount;
                #[cfg(feature = "std")]
                println!(
                    "💰 Order #{}: Processed payment of ${:.2}",
                    order_number, amount
                );
            }
            CashierMessage::Refund {
                amount,
                order_number,
            } => {
                if !self.register_open {
                    #[cfg(feature = "std")]
                    println!(
                        "🚫 Register is closed! Can't process refund for order #{}",
                        order_number
                    );
                    return core::future::ready(());
                }

                self.total_sales -= amount;
                #[cfg(feature = "std")]
                println!("💸 Order #{}: Refunded ${:.2}", order_number, amount);
            }
            CashierMessage::CloseRegister => {
                self.register_open = false;
                #[cfg(feature = "std")]
                println!("🔐 Register closed. Total sales: ${:.2}", self.total_sales);
            }
        }
        core::future::ready(())
    }
}

/// Static storage for the cashier's payment queue.
///
/// This demonstrates:
/// - Static allocation for no_std environments
/// - Memory section placement for embedded systems
/// - Fixed-size queue capacity (like a physical receipt spike)
#[cfg_attr(not(feature = "std"), unsafe(link_section = ".fast_memory"))]
static REGISTER_QUEUE: StaticCell<Queue<CashierMessage, 32>> = StaticCell::new();

/// Example demonstrating the actor system in action.
///
/// This shows a typical day at the coffee shop:
/// - Opening procedures (initialization)
/// - Taking and processing orders (message passing)
/// - Managing breaks and restocking (state changes)
/// - Closing procedures (shutdown)
#[cfg(feature = "std")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("☕ Welcome to the Lit-Bit Coffee Shop! ☕");
    println!("=======================================");

    // Create our barista's order queue using static_mailbox
    println!("\n🏪 Opening the coffee shop...");
    let (mut order_sender, mut order_receiver) =
        static_mailbox!(BARISTA_ORDER_SPIKE: BaristaMessage, 16);
    let mut barista = Barista::new();

    // Create our cashier's queue using create_mailbox
    let (mut payment_sender, mut payment_receiver) = create_mailbox(&REGISTER_QUEUE);
    let mut cashier = Cashier::new();

    // Simulate a busy morning at the coffee shop!
    println!("\n🌅 Morning rush starting...");

    // Customer 1: Orders a latte
    let order_num = 1;
    payment_sender
        .enqueue(CashierMessage::NewOrder {
            amount: 4.50,
            order_number: order_num,
        })
        .unwrap();
    order_sender
        .enqueue(BaristaMessage::MakeDrink {
            drink: DrinkType::Latte,
            extra_shot: false,
            order_number: order_num,
        })
        .unwrap();

    // Customer 2: Orders an espresso with extra shot
    let order_num = 2;
    payment_sender
        .enqueue(CashierMessage::NewOrder {
            amount: 3.00,
            order_number: order_num,
        })
        .unwrap();
    order_sender
        .enqueue(BaristaMessage::MakeDrink {
            drink: DrinkType::Espresso,
            extra_shot: true,
            order_number: order_num,
        })
        .unwrap();

    // Process all pending orders
    println!("\n👩‍🏭 Barista processing orders...");
    while let Some(order) = order_receiver.dequeue() {
        barista.handle(order).await;
    }

    // Process all pending payments
    println!("\n🏦 Cashier processing payments...");
    while let Some(transaction) = payment_receiver.dequeue() {
        cashier.handle(transaction).await;
    }

    // Barista needs a break!
    println!("\n⏰ Time for a break...");
    order_sender.enqueue(BaristaMessage::TakeBreak).unwrap();

    // Customer 3: Tries to order during break
    let order_num = 3;
    payment_sender
        .enqueue(CashierMessage::NewOrder {
            amount: 4.00,
            order_number: order_num,
        })
        .unwrap();
    order_sender
        .enqueue(BaristaMessage::MakeDrink {
            drink: DrinkType::Cappuccino,
            extra_shot: false,
            order_number: order_num,
        })
        .unwrap();

    // Process messages during break
    println!("\n🔄 Processing orders during break...");
    while let Some(order) = order_receiver.dequeue() {
        barista.handle(order).await;
    }
    while let Some(transaction) = payment_receiver.dequeue() {
        cashier.handle(transaction).await;
    }

    // Close up shop
    println!("\n🌙 Closing time...");
    payment_sender
        .enqueue(CashierMessage::CloseRegister)
        .unwrap();
    if let Some(transaction) = payment_receiver.dequeue() {
        cashier.handle(transaction).await;
    }

    println!("\n✨ Coffee shop example completed!");
    Ok(())
}

/// No-std version of the coffee shop example
#[cfg(not(feature = "std"))]
#[unsafe(no_mangle)]
fn main() -> ! {
    // Create actors and mailboxes
    let (order_sender, mut order_receiver) =
        static_mailbox!(BARISTA_ORDER_SPIKE: BaristaMessage, 16);
    let mut barista = Barista::new();

    let (payment_sender, mut payment_receiver) = create_mailbox(&REGISTER_QUEUE);
    let mut cashier = Cashier::new();

    // Main control loop
    loop {
        // Process barista messages
        if let Some(order) = order_receiver.dequeue() {
            let _ = barista.handle(order);
        }

        // Process cashier messages
        if let Some(transaction) = payment_receiver.dequeue() {
            let _ = cashier.handle(transaction);
        }
    }
}
