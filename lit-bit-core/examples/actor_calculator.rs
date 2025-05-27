//! # Basic Actor Example: Calculator
//!
//! This example demonstrates the fundamental concepts of the lit-bit actor system:
//! - Basic actor implementation
//! - Message passing with type safety
//! - Request-response patterns using oneshot channels
//! - Actor lifecycle management
//! - Platform-dual design (works on both std and `no_std`)
//!
//! The calculator actor maintains internal state and responds to arithmetic operations,
//! showcasing how actors encapsulate state and provide controlled access through messages.

use lit_bit_core::actor::{Actor, ActorError};

#[cfg(feature = "std")]
use lit_bit_core::actor::spawn_actor_tokio;

#[cfg(feature = "std")]
use tokio::sync::oneshot;

/// Calculator actor that maintains an internal value and performs arithmetic operations
#[derive(Debug)]
pub struct CalculatorActor {
    value: i32,
    operation_count: u32,
}

impl CalculatorActor {
    #[must_use]
    pub fn new(initial_value: i32) -> Self {
        Self {
            value: initial_value,
            operation_count: 0,
        }
    }
}

/// Messages that the calculator actor can handle
#[derive(Debug)]
pub enum CalcMessage {
    /// Add a number to the current value
    Add(i32),
    /// Subtract a number from the current value
    Subtract(i32),
    /// Multiply the current value by a number
    Multiply(i32),
    /// Divide the current value by a number (returns error if divisor is zero)
    Divide(i32),
    /// Reset the calculator to zero
    Reset,
    /// Get the current value (request-response pattern)
    #[cfg(feature = "std")]
    GetValue { reply_to: oneshot::Sender<i32> },
    /// Get operation statistics
    #[cfg(feature = "std")]
    GetStats {
        reply_to: oneshot::Sender<CalculatorStats>,
    },
}

/// Statistics about calculator operations
#[derive(Debug, Clone)]
pub struct CalculatorStats {
    pub current_value: i32,
    pub operation_count: u32,
}

impl Actor for CalculatorActor {
    type Message = CalcMessage;

    fn on_start(&mut self) -> Result<(), ActorError> {
        #[cfg(feature = "std")]
        println!("🧮 Calculator actor starting with value: {}", self.value);

        // Reset operation count on start
        self.operation_count = 0;
        Ok(())
    }

    fn on_stop(self) -> Result<(), ActorError> {
        #[cfg(feature = "std")]
        println!(
            "🧮 Calculator actor stopping. Final value: {}, Operations: {}",
            self.value, self.operation_count
        );
        Ok(())
    }

    #[cfg(feature = "async")]
    fn on_event(&mut self, msg: CalcMessage) -> futures::future::BoxFuture<'_, ()> {
        Box::pin(async move {
            match msg {
                CalcMessage::Add(n) => {
                    self.value = self.value.saturating_add(n);
                    self.operation_count += 1;
                    #[cfg(feature = "std")]
                    println!("➕ Added {}: result = {}", n, self.value);
                }

                CalcMessage::Subtract(n) => {
                    self.value = self.value.saturating_sub(n);
                    self.operation_count += 1;
                    #[cfg(feature = "std")]
                    println!("➖ Subtracted {}: result = {}", n, self.value);
                }

                CalcMessage::Multiply(n) => {
                    self.value = self.value.saturating_mul(n);
                    self.operation_count += 1;
                    #[cfg(feature = "std")]
                    println!("✖️  Multiplied by {}: result = {}", n, self.value);
                }

                CalcMessage::Divide(n) => {
                    if n != 0 {
                        self.value /= n;
                        self.operation_count += 1;
                        #[cfg(feature = "std")]
                        println!("➗ Divided by {}: result = {}", n, self.value);
                    } else {
                        #[cfg(feature = "std")]
                        println!("❌ Division by zero attempted - operation ignored");
                    }
                }

                CalcMessage::Reset => {
                    self.value = 0;
                    self.operation_count += 1;
                    #[cfg(feature = "std")]
                    println!("🔄 Calculator reset to 0");
                }

                #[cfg(feature = "std")]
                CalcMessage::GetValue { reply_to } => {
                    let _ = reply_to.send(self.value);
                    #[cfg(feature = "std")]
                    println!("📊 Current value requested: {}", self.value);
                }

                #[cfg(feature = "std")]
                CalcMessage::GetStats { reply_to } => {
                    let stats = CalculatorStats {
                        current_value: self.value,
                        operation_count: self.operation_count,
                    };
                    let _ = reply_to.send(stats);
                    #[cfg(feature = "std")]
                    println!(
                        "📈 Stats requested: value={}, operations={}",
                        self.value, self.operation_count
                    );
                }
            }
        })
    }

    #[cfg(not(feature = "async"))]
    fn on_event(&mut self, msg: CalcMessage) -> impl core::future::Future<Output = ()> + Send {
        async move {
            match msg {
                CalcMessage::Add(n) => {
                    self.value = self.value.saturating_add(n);
                    self.operation_count += 1;
                    #[cfg(feature = "std")]
                    println!("➕ Added {}: result = {}", n, self.value);
                }

                CalcMessage::Subtract(n) => {
                    self.value = self.value.saturating_sub(n);
                    self.operation_count += 1;
                    #[cfg(feature = "std")]
                    println!("➖ Subtracted {}: result = {}", n, self.value);
                }

                CalcMessage::Multiply(n) => {
                    self.value = self.value.saturating_mul(n);
                    self.operation_count += 1;
                    #[cfg(feature = "std")]
                    println!("✖️  Multiplied by {}: result = {}", n, self.value);
                }

                CalcMessage::Divide(n) => {
                    if n != 0 {
                        self.value /= n;
                        self.operation_count += 1;
                        #[cfg(feature = "std")]
                        println!("➗ Divided by {}: result = {}", n, self.value);
                    } else {
                        #[cfg(feature = "std")]
                        println!("❌ Division by zero attempted - operation ignored");
                    }
                }

                CalcMessage::Reset => {
                    self.value = 0;
                    self.operation_count += 1;
                    #[cfg(feature = "std")]
                    println!("🔄 Calculator reset to 0");
                }

                #[cfg(feature = "std")]
                CalcMessage::GetValue { reply_to } => {
                    let _ = reply_to.send(self.value);
                    #[cfg(feature = "std")]
                    println!("📊 Current value requested: {}", self.value);
                }

                #[cfg(feature = "std")]
                CalcMessage::GetStats { reply_to } => {
                    let stats = CalculatorStats {
                        current_value: self.value,
                        operation_count: self.operation_count,
                    };
                    let _ = reply_to.send(stats);
                    #[cfg(feature = "std")]
                    println!(
                        "📈 Stats requested: value={}, operations={}",
                        self.value, self.operation_count
                    );
                }
            }
        }
    }
}

#[cfg(feature = "std")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Basic Actor Example: Calculator");
    println!("==================================");

    // Create and spawn the calculator actor
    let calculator = CalculatorActor::new(10);
    let addr = spawn_actor_tokio::<CalculatorActor, 16>(calculator);

    println!("\n🚀 Calculator actor spawned with initial value 10");

    // Perform some calculations
    println!("\n🧮 Performing calculations...");
    addr.send(CalcMessage::Add(5)).await?;
    addr.send(CalcMessage::Multiply(2)).await?;
    addr.send(CalcMessage::Subtract(10)).await?;

    // Get the current value using request-response pattern
    let (tx, rx) = oneshot::channel();
    addr.send(CalcMessage::GetValue { reply_to: tx }).await?;
    let current_value = rx.await?;
    println!("\n📊 Current value: {current_value}");

    // Perform more operations
    println!("\n🧮 More calculations...");
    addr.send(CalcMessage::Divide(2)).await?;
    addr.send(CalcMessage::Add(100)).await?;

    // Test division by zero (should be ignored)
    println!("\n⚠️  Testing division by zero...");
    addr.send(CalcMessage::Divide(0)).await?;

    // Get final statistics
    let (tx, rx) = oneshot::channel();
    addr.send(CalcMessage::GetStats { reply_to: tx }).await?;
    let stats = rx.await?;

    println!("\n📈 Final Statistics:");
    println!("   Value: {}", stats.current_value);
    println!("   Operations: {}", stats.operation_count);

    // Reset and verify
    println!("\n🔄 Resetting calculator...");
    addr.send(CalcMessage::Reset).await?;

    let (tx, rx) = oneshot::channel();
    addr.send(CalcMessage::GetValue { reply_to: tx }).await?;
    let final_value = rx.await?;
    println!("   Value after reset: {final_value}");

    println!("\n✅ Calculator example completed successfully!");
    println!("\n💡 Key Concepts Demonstrated:");
    println!("   • Actor encapsulation of state");
    println!("   • Type-safe message passing");
    println!("   • Request-response patterns with oneshot channels");
    println!("   • Actor lifecycle hooks (on_start, on_stop)");
    println!("   • Error handling (division by zero)");
    println!("   • Saturating arithmetic for overflow protection");

    Ok(())
}

#[cfg(not(feature = "std"))]
fn main() {
    // For no_std targets, this would typically be an embassy-based main
    // or integrated into a larger embedded application
    panic!("This example requires std feature for demonstration purposes");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "std")]
    #[tokio::test]
    async fn test_calculator_basic_operations() {
        let calculator = CalculatorActor::new(0);
        let addr = spawn_actor_tokio::<CalculatorActor, 16>(calculator);

        // Test addition
        addr.send(CalcMessage::Add(10)).await.unwrap();

        let (tx, rx) = oneshot::channel();
        addr.send(CalcMessage::GetValue { reply_to: tx })
            .await
            .unwrap();
        let value = rx.await.unwrap();
        assert_eq!(value, 10);

        // Test multiplication
        addr.send(CalcMessage::Multiply(3)).await.unwrap();

        let (tx, rx) = oneshot::channel();
        addr.send(CalcMessage::GetValue { reply_to: tx })
            .await
            .unwrap();
        let value = rx.await.unwrap();
        assert_eq!(value, 30);
    }

    #[cfg(feature = "std")]
    #[tokio::test]
    async fn test_calculator_division_by_zero() {
        let calculator = CalculatorActor::new(10);
        let addr = spawn_actor_tokio::<CalculatorActor, 16>(calculator);

        // Division by zero should be ignored
        addr.send(CalcMessage::Divide(0)).await.unwrap();

        let (tx, rx) = oneshot::channel();
        addr.send(CalcMessage::GetValue { reply_to: tx })
            .await
            .unwrap();
        let value = rx.await.unwrap();
        assert_eq!(value, 10); // Value should be unchanged
    }

    #[cfg(feature = "std")]
    #[tokio::test]
    async fn test_calculator_reset() {
        let calculator = CalculatorActor::new(42);
        let addr = spawn_actor_tokio::<CalculatorActor, 16>(calculator);

        // Reset should set value to 0
        addr.send(CalcMessage::Reset).await.unwrap();

        let (tx, rx) = oneshot::channel();
        addr.send(CalcMessage::GetValue { reply_to: tx })
            .await
            .unwrap();
        let value = rx.await.unwrap();
        assert_eq!(value, 0);
    }

    #[cfg(feature = "std")]
    #[tokio::test]
    async fn test_calculator_operation_count() {
        let calculator = CalculatorActor::new(0);
        let addr = spawn_actor_tokio::<CalculatorActor, 16>(calculator);

        // Perform several operations
        addr.send(CalcMessage::Add(5)).await.unwrap();
        addr.send(CalcMessage::Multiply(2)).await.unwrap();
        addr.send(CalcMessage::Reset).await.unwrap();

        let (tx, rx) = oneshot::channel();
        addr.send(CalcMessage::GetStats { reply_to: tx })
            .await
            .unwrap();
        let stats = rx.await.unwrap();

        assert_eq!(stats.current_value, 0); // After reset
        assert_eq!(stats.operation_count, 3); // Three operations performed
    }
}
