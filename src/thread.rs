use std::thread;

#[derive(Debug)]
pub(crate) struct Joiner<T>(Vec<thread::JoinHandle<T>>);

impl<T> Joiner<T> {
    pub(crate) fn new<Gen, ThreadFn>(threads: usize, mut thread_fn: Gen) -> Self
    where
        Gen: FnMut() -> ThreadFn,
        ThreadFn: FnOnce() -> T,
        ThreadFn: Send + 'static,
        T: Send + 'static,
    {
        let mut thread_handles = Vec::with_capacity(threads);
        for _ in 0..threads {
            thread_handles.push(std::thread::spawn(thread_fn()));
        }
        Self(thread_handles)
    }

    pub(crate) fn finish(mut self) -> Vec<T> {
        self.0
            .drain(..)
            .map(|handle| handle.join().unwrap())
            .collect()
    }
}

impl<T> Default for Joiner<T> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<T> Drop for Joiner<T> {
    fn drop(&mut self) {
        for t in self.0.drain(..) {
            let res = t.join();
            if !std::thread::panicking() {
                res.unwrap();
            }
        }
    }
}
