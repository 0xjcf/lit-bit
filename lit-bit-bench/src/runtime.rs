//! Runtime-agnostic executor implementations for benchmarking

use futures_lite::future;
use parking_lot::Mutex;
use std::future::Future;
use std::sync::Arc;
use tokio::runtime::Runtime as TokioRuntime;

/// Available runtime types for benchmarking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeType {
    /// Tokio multi-threaded runtime
    TokioMultiThread,
    /// Tokio single-threaded runtime
    TokioSingleThread,
    /// futures-lite minimal executor
    FuturesLite,
    /// Embassy async runtime (when available)
    #[cfg(feature = "embassy")]
    Embassy,
}

/// Runtime-agnostic executor enum
pub enum BenchExecutor {
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
            BenchExecutor::Tokio(e) => e.spawn(future),
            BenchExecutor::FuturesLite(e) => e.spawn(future),
            #[cfg(feature = "embassy")]
            BenchExecutor::Embassy(e) => e.spawn(future),
        }
    }

    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        match self {
            BenchExecutor::Tokio(e) => e.block_on(future),
            BenchExecutor::FuturesLite(e) => e.block_on(future),
            #[cfg(feature = "embassy")]
            BenchExecutor::Embassy(e) => e.block_on(future),
        }
    }

    pub fn runtime_type(&self) -> RuntimeType {
        match self {
            BenchExecutor::Tokio(e) => e.runtime_type,
            BenchExecutor::FuturesLite(_) => RuntimeType::FuturesLite,
            #[cfg(feature = "embassy")]
            BenchExecutor::Embassy(_) => RuntimeType::Embassy,
        }
    }

    pub fn worker_threads(&self) -> Option<usize> {
        match self {
            BenchExecutor::Tokio(e) => e.worker_threads(),
            _ => None,
        }
    }
}

/// Tokio-based executor
pub struct TokioExecutor {
    runtime: Arc<TokioRuntime>,
    runtime_type: RuntimeType,
}

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
            RuntimeType::TokioMultiThread => Some(num_cpus::get()),
            _ => None,
        }
    }
}

/// futures-lite minimal executor
pub struct FuturesLiteExecutor {
    spawn_handle: Arc<Mutex<()>>,
}

impl FuturesLiteExecutor {
    pub fn new() -> Self {
        Self {
            spawn_handle: Arc::new(Mutex::new(())),
        }
    }
}

impl Default for FuturesLiteExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl FuturesLiteExecutor {
    pub fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let _guard = self.spawn_handle.lock();
        future::block_on(future);
    }

    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        future::block_on(future)
    }
}

/// Create a new executor of the specified type
pub fn create_executor(runtime_type: RuntimeType, worker_threads: Option<usize>) -> BenchExecutor {
    match runtime_type {
        RuntimeType::TokioMultiThread => {
            BenchExecutor::Tokio(TokioExecutor::new_multi_thread(worker_threads))
        }
        RuntimeType::TokioSingleThread => BenchExecutor::Tokio(TokioExecutor::new_single_thread()),
        RuntimeType::FuturesLite => BenchExecutor::FuturesLite(FuturesLiteExecutor::new()),
        #[cfg(feature = "embassy")]
        RuntimeType::Embassy => unimplemented!("Embassy runtime not yet implemented"),
    }
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

        let result = executor.block_on(async {
            let mut sum = 0;
            for i in 0..5 {
                sum += i;
            }
            sum
        });
        assert_eq!(result, 10);
    }
}
