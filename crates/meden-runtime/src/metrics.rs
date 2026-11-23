//! Bounded local timing observations. They never authorize or classify effects.
//! Names are static host labels: do not include source, prompts or credentials.
use serde::Serialize;
use std::{
    cell::Cell,
    collections::VecDeque,
    marker::PhantomData,
    rc::Rc,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex, OnceLock,
    },
    time::{Duration, Instant},
};

const CAPACITY: usize = 4096;
static NEXT: AtomicU64 = AtomicU64::new(1);
static STORE: OnceLock<Mutex<Store>> = OnceLock::new();
static ORIGIN: OnceLock<Instant> = OnceLock::new();
thread_local! { static CURRENT: Cell<Option<Context>> = const { Cell::new(None) }; }

/// Correlation only, with no authority or resource identity.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Context {
    trace: u64,
    span: u64,
}

/// One completed wall interval and, when available, its current-thread CPU cost.
#[derive(Clone, Debug, Serialize)]
pub struct Record {
    context: Context,
    parent: Option<u64>,
    name: &'static str,
    start_us: u64,
    wall_us: u64,
    thread_cpu_us: Option<u64>,
}

#[derive(Default)]
struct Store {
    records: VecDeque<Record>,
    evicted: u64,
}

impl Store {
    fn push(&mut self, record: Record) {
        if self.records.len() == CAPACITY {
            self.records.pop_front();
            self.evicted = self.evicted.saturating_add(1);
        }
        self.records.push_back(record);
    }
}

/// A bounded snapshot of completed intervals; open spans appear on completion.
#[derive(Serialize)]
pub struct Snapshot {
    capacity: usize,
    evicted: u64,
    records: Vec<Record>,
}

/// Cheap lexical timer. It cannot cross threads; propagate `context()` instead.
#[must_use = "keep the span alive for the operation being measured"]
pub struct Span {
    context: Context,
    parent: Option<u64>,
    previous: Option<Context>,
    name: &'static str,
    started: Instant,
    cpu: Option<Duration>,
    _thread: PhantomData<Rc<()>>,
}

impl Span {
    /// Start below the current thread's span, or start a new trace.
    pub fn start(name: &'static str) -> Self {
        Self::linked(name, CURRENT.get())
    }

    /// Continue a trace across an application worker boundary.
    pub fn linked(name: &'static str, parent: Option<Context>) -> Self {
        let span = NEXT.fetch_add(1, Ordering::Relaxed);
        let context = Context {
            trace: parent.map_or(span, |p| p.trace),
            span,
        };
        let started = Instant::now();
        ORIGIN.get_or_init(|| started);
        let previous = CURRENT.replace(Some(context));
        // Parent may be on another thread; restoration uses the actual local span.
        Self {
            context,
            parent: parent.map(|p| p.span),
            previous,
            name,
            started,
            cpu: cpu(),
            _thread: PhantomData,
        }
    }

    /// Pass correlation to a worker without keeping this timer alive there.
    pub fn context(&self) -> Context {
        self.context
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        let elapsed = self.started.elapsed();
        let used = self
            .cpu
            .zip(cpu())
            .and_then(|(before, after)| after.checked_sub(before));
        CURRENT.set(self.previous);
        record(Record {
            context: self.context,
            parent: self.parent,
            name: self.name,
            start_us: micros(
                self.started
                    .saturating_duration_since(*ORIGIN.get_or_init(Instant::now)),
            ),
            wall_us: micros(elapsed),
            thread_cpu_us: used.map(micros),
        });
    }
}

/// Record a queue interval without attributing another thread's CPU to it.
pub fn queue(parent: Context, started: Instant) {
    let span = NEXT.fetch_add(1, Ordering::Relaxed);
    record(Record {
        context: Context {
            trace: parent.trace,
            span,
        },
        parent: Some(parent.span),
        name: "job.queue",
        start_us: micros(started.saturating_duration_since(*ORIGIN.get_or_init(Instant::now))),
        wall_us: micros(started.elapsed()),
        thread_cpu_us: None,
    });
}

fn record(record: Record) {
    if let Ok(mut store) = STORE.get_or_init(Mutex::default).lock() {
        store.push(record);
    }
}

/// Copy the ring without resetting it or affecting running work.
pub fn snapshot() -> Snapshot {
    STORE.get_or_init(Mutex::default).lock().map_or_else(
        |_| Snapshot {
            capacity: CAPACITY,
            evicted: 0,
            records: Vec::new(),
        },
        |store| Snapshot {
            capacity: CAPACITY,
            evicted: store.evicted,
            records: store.records.iter().cloned().collect(),
        },
    )
}

fn micros(value: Duration) -> u64 {
    u64::try_from(value.as_micros()).unwrap_or(u64::MAX)
}
#[cfg(unix)]
fn cpu() -> Option<Duration> {
    let value = nix::time::clock_gettime(nix::time::ClockId::CLOCK_THREAD_CPUTIME_ID).ok()?;
    Some(Duration::new(
        u64::try_from(value.tv_sec()).ok()?,
        u32::try_from(value.tv_nsec()).ok()?,
    ))
}
#[cfg(not(unix))]
fn cpu() -> Option<Duration> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_running_hosts_retain_only_the_latest_intervals() {
        let mut store = Store::default();
        for id in 0..u64::try_from(CAPACITY)
            .unwrap_or(u64::MAX)
            .saturating_add(7)
        {
            store.push(Record {
                context: Context {
                    trace: id,
                    span: id,
                },
                parent: None,
                name: "test.bounded",
                start_us: id,
                wall_us: 1,
                thread_cpu_us: None,
            });
        }
        assert_eq!(store.records.len(), CAPACITY);
        assert_eq!(store.evicted, 7);
        assert_eq!(store.records.front().map(|r| r.start_us), Some(7));
    }

    #[test]
    fn nested_failed_and_worker_operations_keep_the_original_trace() {
        let root = Span::start("test.request");
        let context = root.context();
        {
            let _child = Span::start("test.decode_failure");
            let _failed = "invalid".parse::<u64>();
        }
        let queued = Instant::now();
        let joined = std::thread::spawn(move || {
            queue(context, queued);
            let _job = Span::linked("test.worker", Some(context));
            let _io = Span::start("test.io");
        })
        .join();
        assert!(joined.is_ok());
        drop(root);
        let rows = snapshot()
            .records
            .into_iter()
            .filter(|r| r.context.trace == context.trace)
            .collect::<Vec<_>>();
        assert_eq!(rows.len(), 5);
        assert!(rows.iter().all(|r| r.context.trace == context.trace));
        let job = rows.iter().find(|r| r.name == "test.worker");
        let io = rows.iter().find(|r| r.name == "test.io");
        assert_eq!(job.and_then(|r| r.parent), Some(context.span));
        assert_eq!(io.and_then(|r| r.parent), job.map(|r| r.context.span));
        assert!(rows
            .iter()
            .find(|r| r.name == "job.queue")
            .is_some_and(|r| r.thread_cpu_us.is_none()));
        assert!(CURRENT.get().is_none());
    }
}
