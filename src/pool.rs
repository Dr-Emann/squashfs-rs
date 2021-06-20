use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};
use std::{fmt, mem};

pub trait Recyclable {
    fn new() -> Self;
    fn reset(&mut self);
}

impl Recyclable for Vec<u8> {
    fn new() -> Self {
        Vec::new()
    }

    fn reset(&mut self) {
        self.clear();
    }
}

pub struct Pool<T> {
    items: Mutex<Vec<T>>,
}

impl<T: Recyclable> Pool<T> {
    pub fn new(size: usize, capacity: usize) -> Self {
        let mut items = Vec::with_capacity(capacity);
        items.resize_with(size, T::new);
        Self {
            items: Mutex::new(items),
        }
    }

    pub fn detached(&self) -> T {
        self.items.lock().pop().unwrap_or_else(T::new)
    }

    pub fn get(&self) -> Handle<'_, T> {
        Handle {
            value: ManuallyDrop::new(self.detached()),
            pool: self,
        }
    }

    pub fn attach(&self, item: T) -> Handle<'_, T> {
        Handle {
            value: ManuallyDrop::new(item),
            pool: self,
        }
    }

    fn return_item(&self, mut item: T) {
        let mut items = self.items.lock();
        if items.len() < items.capacity() {
            item.reset();
            items.push(item);
        }
    }
}

pub struct Handle<'a, T: Recyclable> {
    value: ManuallyDrop<T>,
    pool: &'a Pool<T>,
}

impl<T: Recyclable> Handle<'_, T> {
    pub fn detach(mut self) -> T {
        let value = unsafe { ManuallyDrop::take(&mut self.value) };
        mem::forget(self);
        value
    }
}

impl<T: Recyclable> Deref for Handle<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T: Recyclable> DerefMut for Handle<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<T: fmt::Debug + Recyclable> fmt::Debug for Handle<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt(f)
    }
}

impl<T: Recyclable> Drop for Handle<'_, T> {
    fn drop(&mut self) {
        let item = unsafe { ManuallyDrop::take(&mut self.value) };
        self.pool.return_item(item);
    }
}

pub type Block<'a> = Handle<'a, Vec<u8>>;

fn blocks() -> &'static Pool<Vec<u8>> {
    static INSTANCE: OnceCell<Pool<Vec<u8>>> = OnceCell::new();

    let threads = num_cpus::get();
    INSTANCE.get_or_init(|| Pool::new(threads, threads * 2))
}

pub fn block() -> Handle<'static, Vec<u8>> {
    blocks().get()
}

pub fn attach_block(block: Vec<u8>) -> Handle<'static, Vec<u8>> {
    blocks().attach(block)
}
