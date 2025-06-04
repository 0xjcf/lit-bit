//! Runtime-agnostic executor implementations for benchmarking

use futures_lite::future;
use std::future::Future;
use std::sync::Arc;
use std::thread;

/// Runtime-specific errors that can occur during executor operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    /// Embassy runtime requires a spawner context and doesn't support direct executor creation
    EmbassyRequiresSpawner { message: String },
    /// Embassy executors don't support synchronous blocking operations
    EmbassyBlockingNotSupported { message: String },
    /// Tokio runtime support is not enabled via feature flags
    TokioNotEnabled {
        requested_runtime: RuntimeType,
        message: String,
    },
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeError::EmbassyRequiresSpawner { message } => {
                write!(f, "Embassy runtime configuration error: {message}")
            }
            RuntimeError::EmbassyBlockingNotSupported { message } => {
                write!(f, "Embassy blocking operation not supported: {message}")
            }
            RuntimeError::TokioNotEnabled {
                requested_runtime,
                message,
            } => {
                write!(
                    f,
                    "Tokio runtime '{requested_runtime:?}' not available: {message}"
                )
            }
        }
    }
}

impl std::error::Error for RuntimeError {}

/// Available runtime types for benchmarking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeType {
    /// Tokio multi-threaded runtime
    #[cfg(feature = "async-tokio")]
    TokioMultiThread,
    /// Tokio single-threaded runtime
    #[cfg(feature = "async-tokio")]
    TokioSingleThread,
    /// futures-lite minimal executor
    FuturesLite,
    /// Embassy async runtime (when available)
    #[cfg(feature = "embassy")]
    Embassy,
}

/// Runtime-agnostic executor enum
pub enum BenchExecutor {
    #[cfg(feature = "async-tokio")]
    Tokio(TokioExecutor),
    FuturesLite(FuturesLiteExecutor),
    #[cfg(feature = "embassy")]
    Embassy(EmbassyExecutor),
}

impl BenchExecutor {
    pub fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        match self {
            #[cfg(feature = "async-tokio")]
            BenchExecutor::Tokio(e) => e.spawn(future),
            BenchExecutor::FuturesLite(e) => e.spawn(future),
            #[cfg(feature = "embassy")]
            BenchExecutor::Embassy(e) => e.spawn(future),
        }
    }

    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        match self {
            #[cfg(feature = "async-tokio")]
            BenchExecutor::Tokio(e) => e.block_on(future),
            BenchExecutor::FuturesLite(e) => e.block_on(future),
            #[cfg(feature = "embassy")]
            BenchExecutor::Embassy(e) => e.block_on(future),
        }
    }

    pub fn runtime_type(&self) -> RuntimeType {
        match self {
            #[cfg(feature = "async-tokio")]
            BenchExecutor::Tokio(e) => e.runtime_type,
            BenchExecutor::FuturesLite(_) => RuntimeType::FuturesLite,
            #[cfg(feature = "embassy")]
            BenchExecutor::Embassy(_) => RuntimeType::Embassy,
        }
    }

    pub fn worker_threads(&self) -> Option<usize> {
        match self {
            #[cfg(feature = "async-tokio")]
            BenchExecutor::Tokio(e) => e.worker_threads(),
            _ => None,
        }
    }
}

impl std::fmt::Debug for BenchExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(feature = "async-tokio")]
            BenchExecutor::Tokio(e) => f.debug_tuple("Tokio").field(e).finish(),
            BenchExecutor::FuturesLite(e) => f.debug_tuple("FuturesLite").field(e).finish(),
            #[cfg(feature = "embassy")]
            BenchExecutor::Embassy(e) => f.debug_tuple("Embassy").field(e).finish(),
        }
    }
}

/// Tokio-based executor
#[cfg(feature = "async-tokio")]
pub struct TokioExecutor {
    runtime: Arc<tokio::runtime::Runtime>,
    runtime_type: RuntimeType,
    worker_threads: Option<usize>,
}

#[cfg(feature = "async-tokio")]
impl TokioExecutor {
    /// Create a new multi-threaded Tokio executor
    pub fn new_multi_thread(worker_threads: Option<usize>) -> Self {
        let mut builder = tokio::runtime::Builder::new_multi_thread();

        if let Some(threads) = worker_threads {
            builder.worker_threads(threads);
        }

        Self {
            runtime: Arc::new(
                builder
                    .enable_all()
                    .build()
                    .expect("Failed to create Tokio runtime"),
            ),
            runtime_type: RuntimeType::TokioMultiThread,
            worker_threads,
        }
    }

    /// Create a new single-threaded Tokio executor
    pub fn new_single_thread() -> Self {
        Self {
            runtime: Arc::new(
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to create Tokio runtime"),
            ),
            runtime_type: RuntimeType::TokioSingleThread,
            worker_threads: None,
        }
    }

    pub fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.runtime.spawn(future);
    }

    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        self.runtime.block_on(future)
    }

    pub fn worker_threads(&self) -> Option<usize> {
        match self.runtime_type {
            RuntimeType::TokioMultiThread => self.worker_threads,
            _ => None,
        }
    }
}

#[cfg(feature = "async-tokio")]
impl std::fmt::Debug for TokioExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokioExecutor")
            .field("runtime_type", &self.runtime_type)
            .field("worker_threads", &self.worker_threads)
            // Skip runtime field as it doesn't need to be debugged
            .finish()
    }
}

/// futures-lite minimal executor with proper concurrent spawning
pub struct FuturesLiteExecutor {
    executor: Arc<async_executor::Executor<'static>>,
    _executor_thread: thread::JoinHandle<()>,
}

impl FuturesLiteExecutor {
    pub fn new() -> Self {
        let executor = Arc::new(async_executor::Executor::new());
        let executor_clone = executor.clone();

        // Spawn a background thread to drive the executor
        let executor_thread = thread::spawn(move || {
            // Run the executor forever, processing spawned tasks
            future::block_on(executor_clone.run(future::pending::<()>()));
        });

        Self {
            executor,
            _executor_thread: executor_thread,
        }
    }
}

impl Default for FuturesLiteExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl FuturesLiteExecutor {
    /// Spawn a future concurrently (non-blocking)
    pub fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        // Now properly spawns the future without blocking
        self.executor.spawn(future).detach();
    }

    /// Block on a future until completion
    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        future::block_on(future)
    }
}

impl std::fmt::Debug for FuturesLiteExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FuturesLiteExecutor")
            // Skip internal executor and thread as they're not relevant for debugging
            .finish_non_exhaustive()
    }
}

/// Embassy-based executor with proper concurrent spawning
#[cfg(feature = "embassy")]
pub struct EmbassyExecutor {
    executor: Arc<async_executor::Executor<'static>>,
    _executor_thread: thread::JoinHandle<()>,
    _spawner: embassy_executor::Spawner,
}

#[cfg(feature = "embassy")]
impl EmbassyExecutor {
    pub fn new(spawner: embassy_executor::Spawner) -> Self {
        let executor = Arc::new(async_executor::Executor::new());
        let executor_clone = executor.clone();

        // Spawn a background thread to drive the executor
        let executor_thread = thread::spawn(move || {
            // Run the executor forever, processing spawned tasks
            future::block_on(executor_clone.run(future::pending::<()>()));
        });

        Self {
            executor,
            _executor_thread: executor_thread,
            _spawner: spawner,
        }
    }

    /// Spawn a future concurrently (non-blocking)
    pub fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        // Now properly spawns the future without blocking
        self.executor.spawn(future).detach();
    }

    pub fn block_on<F: Future>(&self, _future: F) -> F::Output {
        let error = RuntimeError::EmbassyBlockingNotSupported {
            message: "Embassy executors are designed for embedded environments and don't support synchronous blocking operations. Use the Embassy executor's main function or spawn tasks instead.".to_string(),
        };
        panic!("{error}");
    }
}

#[cfg(feature = "embassy")]
impl std::fmt::Debug for EmbassyExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmbassyExecutor")
            // Skip internal executor, thread, and spawner as they're not relevant for debugging
            .finish_non_exhaustive()
    }
}

/// Create a new executor of the specified type
pub fn create_executor(runtime_type: RuntimeType, worker_threads: Option<usize>) -> BenchExecutor {
    match runtime_type {
        #[cfg(feature = "async-tokio")]
        RuntimeType::TokioMultiThread => {
            BenchExecutor::Tokio(TokioExecutor::new_multi_thread(worker_threads))
        }
        #[cfg(feature = "async-tokio")]
        RuntimeType::TokioSingleThread => BenchExecutor::Tokio(TokioExecutor::new_single_thread()),
        RuntimeType::FuturesLite => BenchExecutor::FuturesLite(FuturesLiteExecutor::new()),
        #[cfg(feature = "embassy")]
        RuntimeType::Embassy => {
            let error = RuntimeError::EmbassyRequiresSpawner {
                message: "Embassy runtime requires a spawner from an Embassy executor context. Embassy executors must be created within an Embassy runtime using embassy_executor::Spawner::new(). Consider using create_embassy_executor_with_spawner() instead.".to_string(),
            };
            panic!("{error}");
        }
        #[cfg(not(feature = "async-tokio"))]
        RuntimeType::TokioMultiThread | RuntimeType::TokioSingleThread => {
            let error = RuntimeError::TokioNotEnabled {
                requested_runtime: runtime_type,
                message:
                    "Enable the 'async-tokio' feature in your Cargo.toml to use Tokio runtimes."
                        .to_string(),
            };
            panic!("{error}");
        }
    }
}

/// Create an Embassy executor with the provided spawner
///
/// This is the proper way to create Embassy executors, as they require
/// a spawner context from an Embassy runtime environment.
#[cfg(feature = "embassy")]
pub fn create_embassy_executor_with_spawner(spawner: embassy_executor::Spawner) -> BenchExecutor {
    BenchExecutor::Embassy(EmbassyExecutor::new(spawner))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_tokio_multi_thread() {
        let executor = create_executor(RuntimeType::TokioMultiThread, Some(2));
        assert_eq!(executor.runtime_type(), RuntimeType::TokioMultiThread);
        assert_eq!(executor.worker_threads(), Some(2));

        executor.block_on(async {
            let (tx, rx) = tokio::sync::oneshot::channel();
            executor.spawn(async move {
                tokio::time::sleep(Duration::from_millis(10)).await;
                tx.send(42).unwrap();
            });
            assert_eq!(rx.await.unwrap(), 42);
        });
    }

    #[test]
    fn test_tokio_single_thread() {
        let executor = create_executor(RuntimeType::TokioSingleThread, None);
        assert_eq!(executor.runtime_type(), RuntimeType::TokioSingleThread);
        assert_eq!(executor.worker_threads(), None);

        executor.block_on(async {
            let (tx, rx) = tokio::sync::oneshot::channel();
            executor.spawn(async move {
                tx.send(42).unwrap();
            });
            assert_eq!(rx.await.unwrap(), 42);
        });
    }

    #[test]
    fn test_futures_lite() {
        let executor = create_executor(RuntimeType::FuturesLite, None);
        assert_eq!(executor.runtime_type(), RuntimeType::FuturesLite);
        assert_eq!(executor.worker_threads(), None);

        // Test concurrent spawning with shared state
        executor.block_on(async {
            use std::sync::atomic::{AtomicUsize, Ordering};
            use std::time::Duration;

            let counter = Arc::new(AtomicUsize::new(0));
            let start_time = std::time::Instant::now();

            // Spawn multiple futures that should run concurrently
            for _ in 0..3 {
                let counter_clone = counter.clone();
                executor.spawn(async move {
                    // Simulate some async work
                    futures_lite::future::yield_now().await;
                    counter_clone.fetch_add(1, Ordering::SeqCst);
                });
            }

            // Wait for all spawned tasks to complete
            // In a proper concurrent implementation, this should not take long
            loop {
                futures_lite::future::yield_now().await;
                if counter.load(Ordering::SeqCst) == 3 {
                    break;
                }
                // Add a timeout to prevent infinite loops in tests
                if start_time.elapsed() > Duration::from_secs(5) {
                    panic!("Test timed out - spawned tasks may not be running concurrently");
                }
            }

            assert_eq!(counter.load(Ordering::SeqCst), 3);
        });
    }

    #[test]
    #[should_panic(expected = "Embassy runtime configuration error")]
    fn test_embassy_executor_creation_without_spawner_gives_descriptive_error() {
        // This should panic with a descriptive error message instead of a generic one
        let _executor = create_executor(RuntimeType::Embassy, None);
    }

    #[test]
    fn test_runtime_error_display_formatting() {
        let embassy_spawner_error = RuntimeError::EmbassyRequiresSpawner {
            message: "Test message".to_string(),
        };
        assert!(
            embassy_spawner_error
                .to_string()
                .contains("Embassy runtime configuration error")
        );
        assert!(embassy_spawner_error.to_string().contains("Test message"));

        let embassy_blocking_error = RuntimeError::EmbassyBlockingNotSupported {
            message: "Blocking not supported".to_string(),
        };
        assert!(
            embassy_blocking_error
                .to_string()
                .contains("Embassy blocking operation not supported")
        );

        let tokio_error = RuntimeError::TokioNotEnabled {
            requested_runtime: RuntimeType::FuturesLite, // Use a valid variant for the test
            message: "Feature not enabled".to_string(),
        };
        assert!(tokio_error.to_string().contains("not available"));
    }
}
