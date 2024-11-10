use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, RwLock,
    },
    time::Duration,
};

use crate::{WasiProcess, WasiProcessId};
use wasmer_types::ModuleHash;

#[derive(Debug, Clone)]
pub struct WasiControlPlane {
    state: Arc<State>,
}

#[derive(Debug, Clone)]
pub struct WasiControlPlaneHandle {
    inner: std::sync::Weak<State>,
}

impl WasiControlPlaneHandle {
    fn new(inner: &Arc<State>) -> Self {
        Self {
            inner: Arc::downgrade(inner),
        }
    }

    pub fn upgrade(&self) -> Option<WasiControlPlane> {
        self.inner.upgrade().map(|state| WasiControlPlane { state })
    }

    pub fn must_upgrade(&self) -> WasiControlPlane {
        let state = self.inner.upgrade().expect("control plane unavailable");
        WasiControlPlane { state }
    }
}

#[derive(Debug, Clone)]
pub struct ControlPlaneConfig {
    /// Total number of tasks (processes + threads) that can be spawned.
    pub max_task_count: Option<usize>,
    /// Flag that indicates if asynchronous threading is enables (opt-in)
    pub enable_asynchronous_threading: bool,
    /// Enables an exponential backoff of the process CPU usage when there
    /// are no active run tokens (when set holds the maximum amount of
    /// time that it will pause the CPU)
    /// (default = off)
    pub enable_exponential_cpu_backoff: Option<Duration>,
}

impl ControlPlaneConfig {
    pub fn new() -> Self {
        Self {
            max_task_count: None,
            enable_asynchronous_threading: false,
            enable_exponential_cpu_backoff: None,
        }
    }
}

impl Default for ControlPlaneConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
struct State {
    /// Total number of active tasks (threads) across all processes.
    task_count: Arc<AtomicUsize>,

    /// Mutable state.
    mutable: RwLock<MutableState>,
}

#[derive(Debug)]
struct MutableState {
    /// Seed used to generate process ID's
    process_seed: u32,
    /// The processes running on this machine
    processes: HashMap<WasiProcessId, WasiProcess>,
    // TODO: keep a queue of terminated process ids for id reuse.
}

impl WasiControlPlane {
    pub fn new() -> Self {
        Self {
            state: Arc::new(State {
                task_count: Arc::new(AtomicUsize::new(0)),
                mutable: RwLock::new(MutableState {
                    process_seed: 0,
                    processes: Default::default(),
                }),
            }),
        }
    }

    pub fn handle(&self) -> WasiControlPlaneHandle {
        WasiControlPlaneHandle::new(&self.state)
    }

    /// Get the current count of active tasks (threads).
    fn active_task_count(&self) -> usize {
        self.state.task_count.load(Ordering::SeqCst)
    }

    /// Register a new task.
    ///
    // Currently just increments the task counter.
    pub(crate) fn register_task(&self) -> Result<TaskCountGuard, ControlPlaneError> {
        self.state.task_count.fetch_add(1, Ordering::SeqCst);
        Ok(TaskCountGuard(self.state.task_count.clone()))
    }

    /// Creates a new process
    // FIXME: De-register terminated processes!
    // Currently they just accumulate.
    pub fn new_process(&self, module_hash: ModuleHash) -> Result<WasiProcess, ControlPlaneError> {
        // Create the process first to do all the allocations before locking.
        let mut proc = WasiProcess::new(WasiProcessId::from(0), module_hash, self.handle());

        let mut mutable = self.state.mutable.write().unwrap();

        let pid = mutable.next_process_id()?;
        proc.set_pid(pid);
        mutable.processes.insert(pid, proc.clone());
        Ok(proc)
    }

    /// Generates a new process ID
    pub fn generate_id(&self) -> Result<WasiProcessId, ControlPlaneError> {
        let mut mutable = self.state.mutable.write().unwrap();
        mutable.next_process_id()
    }

    /// Gets a reference to a running process
    pub fn get_process(&self, pid: WasiProcessId) -> Option<WasiProcess> {
        self.state
            .mutable
            .read()
            .unwrap()
            .processes
            .get(&pid)
            .cloned()
    }
}

impl MutableState {
    fn next_process_id(&mut self) -> Result<WasiProcessId, ControlPlaneError> {
        // TODO: reuse terminated ids, handle wrap-around, ...
        let id = self.process_seed.checked_add(1).ok_or({
            ControlPlaneError::TaskLimitReached {
                max: u32::MAX as usize,
            }
        })?;
        self.process_seed = id;
        Ok(WasiProcessId::from(id))
    }
}

impl Default for WasiControlPlane {
    fn default() -> Self {
        Self::new()
    }
}

/// Guard that ensures the [`WasiControlPlane`] task counter is decremented when dropped.
#[derive(Debug)]
pub struct TaskCountGuard(Arc<AtomicUsize>);

impl Drop for TaskCountGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

#[derive(thiserror::Error, PartialEq, Eq, Clone, Debug)]
pub enum ControlPlaneError {
    /// The maximum number of execution tasks has been reached.
    #[error("The maximum number of execution tasks has been reached ({max})")]
    TaskLimitReached {
        /// The maximum number of tasks.
        max: usize,
    },
}
