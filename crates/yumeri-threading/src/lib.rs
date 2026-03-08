mod channel;
mod pool;
mod scoped;
mod task;

use std::sync::OnceLock;

pub use channel::{
    bounded, unbounded, select, Receiver, RecvError, SendError, Sender, TryRecvError, TrySendError,
};
pub use pool::ThreadPool;
pub use scoped::{par_for_each, par_map};
pub use task::{Task, TaskError, TaskStatus};

pub(crate) fn parallelism() -> usize {
    static N: OnceLock<usize> = OnceLock::new();
    *N.get_or_init(|| {
        std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(4)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_spawn_and_join() {
        let pool = ThreadPool::new(2);
        let (tx, rx) = bounded(1);
        pool.spawn(move || {
            tx.send(42).unwrap();
        });
        assert_eq!(rx.recv().unwrap(), 42);
    }

    #[test]
    fn pool_spawn_task_returns_value() {
        let pool = ThreadPool::new(2);
        let task = pool.spawn_task(|| 123);
        let result = task.wait().unwrap();
        assert_eq!(result, 123);
    }

    #[test]
    fn task_try_get_lifecycle() {
        let pool = ThreadPool::new(1);
        let mut task = pool.spawn_task(|| {
            std::thread::sleep(std::time::Duration::from_millis(50));
            "hello"
        });

        assert_eq!(task.status(), TaskStatus::Pending);

        std::thread::sleep(std::time::Duration::from_millis(200));
        assert_eq!(task.try_get(), Some(&"hello"));
        assert_eq!(task.status(), TaskStatus::Ready);
        assert!(task.is_ready());

        let val = task.take().unwrap();
        assert_eq!(val, "hello");
    }

    #[test]
    fn task_handles_panic() {
        let pool = ThreadPool::new(1);
        let mut task: Task<()> = pool.spawn_task(|| {
            panic!("test panic");
        });

        std::thread::sleep(std::time::Duration::from_millis(200));
        task.poll();
        assert_eq!(task.status(), TaskStatus::Failed);
        assert!(task.take().is_none());
    }

    #[test]
    fn pool_survives_task_panic() {
        let pool = ThreadPool::new(2);

        let _task: Task<()> = pool.spawn_task(|| {
            panic!("boom");
        });
        std::thread::sleep(std::time::Duration::from_millis(100));

        let task2 = pool.spawn_task(|| 99);
        let result = task2.wait().unwrap();
        assert_eq!(result, 99);
    }

    #[test]
    fn pool_default_size() {
        let pool = ThreadPool::with_default_size();
        assert!(pool.thread_count() >= 2);
        assert!(pool.thread_count() <= 16);
    }

    #[test]
    fn par_map_preserves_order() {
        let items: Vec<i32> = (0..100).collect();
        let result = par_map(&items, 4, |x| x * 2);
        let expected: Vec<i32> = (0..100).map(|x| x * 2).collect();
        assert_eq!(result, expected);
    }

    #[test]
    fn par_map_small_batch_single_threaded() {
        let items = vec![1, 2, 3];
        let result = par_map(&items, 10, |x| x + 1);
        assert_eq!(result, vec![2, 3, 4]);
    }

    #[test]
    fn par_for_each_processes_all() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let items: Vec<i32> = (0..50).collect();
        let sum = AtomicUsize::new(0);
        par_for_each(&items, 4, |x| {
            sum.fetch_add(*x as usize, Ordering::Relaxed);
        });
        assert_eq!(sum.load(Ordering::Relaxed), (0..50).sum::<i32>() as usize);
    }

    #[test]
    fn par_for_each_empty_slice() {
        let items: Vec<i32> = vec![];
        par_for_each(&items, 4, |_| {
            panic!("should not be called");
        });
    }

    #[test]
    fn task_wait_consumes() {
        let pool = ThreadPool::new(1);
        let task = pool.spawn_task(|| vec![1, 2, 3]);
        let result = task.wait().unwrap();
        assert_eq!(result, vec![1, 2, 3]);
    }
}
