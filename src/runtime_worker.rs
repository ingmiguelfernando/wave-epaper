//! Reusable short-lived worker boundary.
//!
//! Heavy operations receive named stack budgets and return compact heap-owned
//! results to the main hardware-orchestration loop. Panel SPI ownership stays
//! on the main task.

use core::fmt::{self, Display};

#[derive(Debug)]
pub enum NamedWorkerError<E> {
    Start(std::io::Error),
    Panicked,
    Operation(E),
}

impl<E: Display> Display for NamedWorkerError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Start(error) if error.raw_os_error() == Some(ENOMEM) => {
                formatter.write_str("not enough free memory right now; try again")
            }
            Self::Start(error) => write!(formatter, "worker start failed: {error}"),
            Self::Panicked => formatter.write_str("worker panicked"),
            Self::Operation(error) => Display::fmt(error, formatter),
        }
    }
}

impl<E: Display + fmt::Debug> std::error::Error for NamedWorkerError<E> {}

/// `errno` when a thread stack cannot be allocated.
const ENOMEM: i32 = 12;

/// A photo still being prepared holds internal memory for a few seconds, so
/// wait up to three seconds for a block that fits the stack.
#[cfg(target_os = "espidf")]
pub(crate) fn wait_for_stack_memory(stack_bytes: usize) {
    use esp_idf_svc::sys;
    for _ in 0..12 {
        let largest = unsafe {
            sys::heap_caps_get_largest_free_block(
                (sys::MALLOC_CAP_INTERNAL | sys::MALLOC_CAP_8BIT) as u32,
            )
        };
        if largest > stack_bytes + 1024 {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
}

#[cfg(not(target_os = "espidf"))]
pub(crate) fn wait_for_stack_memory(_stack_bytes: usize) {}

pub fn run_named_worker<T, E, F>(
    name: &'static str,
    stack_bytes: usize,
    task: F,
) -> Result<T, NamedWorkerError<E>>
where
    T: Send + 'static,
    E: Display + Send + 'static,
    F: FnOnce() -> Result<T, E> + Send + 'static,
{
    log::info!(
        "rustmix-wave=worker-boundary name={name} status=starting stack-bytes={stack_bytes}"
    );
    crate::runtime_memory::log_runtime_memory(&format!("before-worker-{name}"));
    wait_for_stack_memory(stack_bytes);
    let worker = std::thread::Builder::new()
        .name(name.into())
        .stack_size(stack_bytes)
        .spawn(task)
        .map_err(|error| {
            log::warn!(
                "rustmix-wave=worker-boundary name={name} status=start-failed error={error}"
            );
            NamedWorkerError::Start(error)
        })?;
    let result = worker.join().map_err(|_| {
        log::warn!("rustmix-wave=worker-boundary name={name} status=panicked");
        NamedWorkerError::Panicked
    })?;
    crate::runtime_memory::log_runtime_memory(&format!("after-worker-{name}"));
    match result {
        Ok(value) => {
            log::info!("rustmix-wave=worker-boundary name={name} status=completed");
            Ok(value)
        }
        Err(error) => {
            log::warn!("rustmix-wave=worker-boundary name={name} status=failed error={error}");
            Err(NamedWorkerError::Operation(error))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{run_named_worker, NamedWorkerError};

    #[test]
    fn returns_compact_result_from_named_short_lived_worker() {
        let result = run_named_worker("unit-worker", 16 * 1024, || Ok::<_, String>(42)).unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn a_start_without_memory_asks_to_try_again() {
        let error = std::io::Error::from_raw_os_error(12);
        let message = NamedWorkerError::<String>::Start(error).to_string();
        assert_eq!(message, "not enough free memory right now; try again");
    }
}
