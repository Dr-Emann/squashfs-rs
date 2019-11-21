#[cfg(feature = "packed_serialize_derive")]
#[allow(unused_imports)]
#[macro_use]
extern crate packed_serialize_derive;

#[cfg(feature = "packed_serialize_derive")]
#[doc(hidden)]
pub use packed_serialize_derive::*;

use std::io;
use std::mem;
use std::ptr;

/// Re-export
pub use generic_array;

use generic_array::typenum::consts::*;
use generic_array::typenum::Unsigned;
use generic_array::{ArrayLength, GenericArray};

#[inline]
pub fn read<T: PackedStruct, R: io::Read>(mut reader: R) -> io::Result<T> {
    let mut buf: GenericArray<u8, T::Size> = unsafe { mem::MaybeUninit::uninit().assume_init() };
    reader.read_exact(&mut buf[..])?;
    Ok(T::from_packed(&buf))
}

/// Attempt to read a packed struct from reader
///
/// If the reader is empty, returns Ok(None), otherwise, attempts to read
/// exactly `Size` bytes from the reader, and converts to a T
///
/// # Examples
/// ```
/// use packed_serialize::PackedStruct;
///
/// #[derive(Debug, PackedStruct)]
/// struct MyStruct {
///     x: u16,
///     y: u16,
/// }
///
/// let empty_data: &[u8] = &[];
/// assert!(packed_serialize::try_read::<MyStruct, _>(empty_data).unwrap().is_none());
///
/// let short_data: &[u8] = &[0x01, 0x02];
/// let short_result: std::io::Error = packed_serialize::try_read::<MyStruct, _>(short_data).unwrap_err();
/// assert_eq!(short_result.kind(), std::io::ErrorKind::UnexpectedEof);
///
/// let full_data: &[u8] = &[0xDE, 0xAD, 0xBE, 0xEF];
/// let my_struct: MyStruct = packed_serialize::try_read(full_data).unwrap().unwrap();
/// assert_eq!(my_struct.x, 0xADDE);
/// assert_eq!(my_struct.y, 0xEFBE);
/// ```
pub fn try_read<T: PackedStruct, R: io::Read>(mut reader: R) -> io::Result<Option<T>> {
    let mut buf: GenericArray<u8, T::Size> = unsafe { mem::MaybeUninit::uninit().assume_init() };
    let mut slice = &mut buf[..];
    // Try to read until we get a non-interrupted read.
    // If the first non-interrupted read is EOF, return None,
    // Otherwise, expect exactly the remaining size of the buffer
    loop {
        match reader.read(slice) {
            Ok(0) => return Ok(None),
            Ok(n) => {
                slice = &mut slice[n..];
                break;
            }
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    reader.read_exact(&mut slice)?;
    Ok(Some(T::from_packed(&buf)))
}

#[inline]
pub fn write<T: PackedStruct, W: io::Write>(s: &T, mut writer: W) -> io::Result<()> {
    let bytes = s.to_packed();
    writer.write_all(&bytes)
}

pub trait PackedStruct {
    type Size: ArrayLength<u8>;

    #[inline]
    fn size() -> usize {
        Self::Size::to_usize()
    }

    fn to_packed(&self) -> GenericArray<u8, Self::Size> {
        let mut arr: GenericArray<u8, Self::Size> =
            unsafe { mem::MaybeUninit::uninit().assume_init() };
        self.write_packed_arr(&mut arr);
        arr
    }

    fn from_packed(array: &GenericArray<u8, Self::Size>) -> Self
    where
        Self: Sized;
    fn read_packed_arr(&mut self, arr: &GenericArray<u8, Self::Size>);
    fn write_packed_arr(&self, arr: &mut GenericArray<u8, Self::Size>);
}

macro_rules! packed_impl {
    ($ty_name:ty, $size:ty) => {
        impl PackedStruct for $ty_name {
            type Size = $size;

            #[inline]
            fn from_packed(array: &GenericArray<u8, Self::Size>) -> Self {
                <$ty_name>::from_le(unsafe {
                    #[allow(clippy::cast_ptr_alignment)]
                    ptr::read_unaligned(array.as_slice().as_ptr() as *const Self)
                })
            }

            #[inline]
            fn read_packed_arr(&mut self, array: &GenericArray<u8, Self::Size>) {
                *self = Self::from_packed(array);
            }

            #[inline]
            fn write_packed_arr(&self, array: &mut GenericArray<u8, Self::Size>) {
                unsafe {
                    #[allow(clippy::cast_ptr_alignment)]
                    ptr::write_unaligned(array.as_mut_slice().as_ptr() as *mut Self, self.to_le())
                }
            }
        }
    };
}

packed_impl!(u8, U1);
packed_impl!(i8, U1);
packed_impl!(u16, U2);
packed_impl!(i16, U2);
packed_impl!(u32, U4);
packed_impl!(i32, U4);
packed_impl!(u64, U8);
packed_impl!(i64, U8);
packed_impl!(u128, U16);
packed_impl!(i128, U16);
