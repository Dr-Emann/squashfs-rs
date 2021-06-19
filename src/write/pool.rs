use object_pool::Pool;
use once_cell::sync::OnceCell;

fn metablock_pool() -> &'static Pool<Vec<u8>> {
    static INSTANCE: OnceCell<Pool<Vec<u8>>> = OnceCell::new();

    INSTANCE.get_or_init(|| {
        Pool::new(num_cpus::get() * 3 / 2, || {
            Vec::with_capacity(repr::metablock::SIZE)
        })
    })
}

fn datablock_pool() -> &'static Pool<Vec<u8>> {
    static INSTANCE: OnceCell<Pool<Vec<u8>>> = OnceCell::new();

    INSTANCE.get_or_init(|| Pool::new(num_cpus::get(), || Vec::with_capacity(1024 * 1024)))
}

pub(crate) type Handle = object_pool::Reusable<'static, Vec<u8>>;

pub(crate) fn metablock() -> Handle {
    let mut data = metablock_pool().pull(|| Vec::with_capacity(repr::metablock::SIZE));
    data.clear();
    data
}

pub(crate) fn datablock() -> Handle {
    let mut data = datablock_pool().pull(|| Vec::with_capacity(1024 * 1024));
    data.clear();
    data
}

pub(crate) fn attach(data: Vec<u8>) {
    let capacity = data.capacity();
    let pool = if capacity <= repr::metablock::SIZE {
        metablock_pool()
    } else if capacity <= 1024 * 1024 {
        datablock_pool()
    } else {
        return;
    };
    pool.attach(data);
}
