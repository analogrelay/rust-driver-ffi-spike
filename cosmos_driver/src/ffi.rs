use std::{marker::PhantomData, mem::MaybeUninit};

use crate::{Error, Pipeline};

#[repr(i8)]
#[allow(dead_code)]
pub enum ResultCode {
    /// The call succeeded
    Success = 0,

    /// The call succeeded, but there was no data to return
    None = 1,

    /// The caller passed an invalid argument, such as a null pointer
    InvalidArgument = -1,

    /// The caller passed an a string that is not valid UTF-8
    InvalidString = -2,

    /// The caller passed an a string that is not valid JSON or doesn't match the expected schema
    InvalidJson = -3,

    /// The caller passed a null pointer
    NullPointer = -4,

    /// The caller passed an improperly aligned pointer
    UnalignedPointer = -5,

    /// The caller referenced a partition that does not exist
    PartitionDoesNotExist = -6,
}

impl From<Error> for ResultCode {
    fn from(e: Error) -> Self {
        match e {
            Error::InvalidJson(_) => ResultCode::InvalidJson,
            Error::PartitionDoesNotExist => ResultCode::PartitionDoesNotExist,
        }
    }
}

impl<T> From<Result<T, Error>> for ResultCode {
    fn from(e: Result<T, Error>) -> Self {
        match e {
            Ok(_) => ResultCode::Success,
            Err(e) => e.into(),
        }
    }
}

/// A version of '?' that returns FfiResult instead of Result.
macro_rules! ffi_try {
    ($e:expr) => {
        match $e {
            Ok(value) => value,
            Err(error) => return error.into(),
        }
    };
}

/// An FFI-safe representation of bytes **owned by the driver**.
///
/// A caller who recieves this value is expected to call bytes_free to release the memory when they're done with them.
#[repr(C)]
pub struct FfiBytes {
    data: *const u8,
    len: usize,
}

impl FfiBytes {
    pub fn new(buffer: Box<[u8]>) -> Self {
        let ptr = Box::leak(buffer);
        Self {
            data: ptr.as_ptr(),
            len: ptr.len(),
        }
    }

    pub unsafe fn into_box(self) -> Box<[u8]> {
        Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            self.data as *mut u8,
            self.len,
        ))
    }
}

/// An FFI-safe slice of values OWNED by the caller.
///
/// Functions which receive this should never store it. It comes from an FFI call and the caller is free to release the memory it points to immediately upon returning from the call.
#[repr(C)]
pub struct FfiSlice<'a> {
    data: *const (),
    len: usize,
    phantom: PhantomData<&'a ()>,
}

impl<'a> FfiSlice<'a> {
    pub unsafe fn to_str(&self) -> Result<&'a str, ResultCode> {
        // SAFETY: The caller is asserting that this pointer is a valid pointer to a string.
        self.to_slice()
            .and_then(|s| std::str::from_utf8(s).map_err(|_| ResultCode::InvalidString))
    }

    pub unsafe fn to_string(&self) -> Result<String, ResultCode> {
        // SAFETY: The caller is asserting that this pointer is a valid pointer to a string.
        self.to_str().map(|s| s.to_string())
    }

    pub unsafe fn to_slice<T>(&self) -> Result<&'a [T], ResultCode> {
        let ptr = self.data as *const T;
        if ptr.is_null() {
            return Err(ResultCode::NullPointer);
        }

        if !ptr.is_aligned() {
            return Err(ResultCode::UnalignedPointer);
        }

        unsafe { Ok(std::slice::from_raw_parts(ptr, self.len)) }
    }
}

/// An FFI-safe result.
///
/// If the `code` value is [`ResultCode::Success`], then [`value`] is guaranteed to be initialized. If the "code" value is anything else, then [`value`] is guaranteed to be uninitialized.
/// If the `code` value is [`ResultCode::None`], then [`value`] is guaranteed to be uninitialized. This represents a successfull call with no data to return.
///
/// The Rust implementation is configured to enforce these rules.
#[repr(C)]
pub struct FfiResult<T> {
    code: ResultCode,
    value: MaybeUninit<T>,
}

impl<T> FfiResult<T> {
    pub fn ok(value: T) -> Self {
        Self {
            code: ResultCode::Success,
            value: MaybeUninit::new(value),
        }
    }

    pub fn none() -> Self {
        Self {
            code: ResultCode::None,
            value: MaybeUninit::uninit(),
        }
    }

    pub fn err(error: ResultCode) -> Self {
        Self {
            code: error,
            value: MaybeUninit::uninit(),
        }
    }
}

impl<T> From<Result<T, Error>> for FfiResult<T> {
    fn from(result: Result<T, Error>) -> Self {
        match result {
            Ok(value) => Self::ok(value),
            Err(error) => Self::err(error.into()),
        }
    }
}

impl<T> From<Result<T, ResultCode>> for FfiResult<T> {
    fn from(result: Result<T, ResultCode>) -> Self {
        match result {
            Ok(value) => Self::ok(value),
            Err(error) => Self::err(error),
        }
    }
}

impl<T> From<ResultCode> for FfiResult<T> {
    fn from(result: ResultCode) -> Self {
        FfiResult::err(result)
    }
}

impl<T> From<Error> for FfiResult<T> {
    fn from(result: Error) -> Self {
        FfiResult::err(result.into())
    }
}

impl<T> From<Option<T>> for FfiResult<T> {
    fn from(v: Option<T>) -> Self {
        match v {
            Some(v) => FfiResult::ok(v),
            None => FfiResult::none(),
        }
    }
}

/// Creates a new pipeline, given the partition IDs.
///
/// The strings provided will be copied in to Rust-owned strings, so the caller need not preserve their lifetime beyond the call to this function.
#[no_mangle]
extern "C" fn pipeline_new<'a>(partition_ids: FfiSlice<'a>) -> FfiResult<*mut Pipeline> {
    // SAFETY: The caller is asserting that this pointer is a valid slice of strings.
    let partition_ids: &[FfiSlice<'a>] = ffi_try!(unsafe { partition_ids.to_slice() });

    // Copy all the provided strings over to Rust-owned strings.
    let mut ids = Vec::with_capacity(partition_ids.len());
    for partition_id in partition_ids.iter() {
        // SAFETY: The caller is asserting that this pointer is a valid pointer to a string.
        let id = ffi_try!(unsafe { partition_id.to_string() });
        ids.push(id)
    }

    // Create the pipeline on the heap, and leak it. The caller will give it back to us to free, hopefully.
    let pipeline = Box::new(Pipeline::new(ids));
    FfiResult::ok(std::ptr::from_mut(Box::leak(pipeline)))
}

/// Frees any single object provided by the driver.
#[no_mangle]
extern "C" fn pipeline_free(value: *mut Pipeline) -> () {
    // Take the value back from the caller, and drop it.
    unsafe { drop(Box::from_raw(value)) };
}

/// Frees any single object provided by the driver.
#[no_mangle]
extern "C" fn bytes_free(value: FfiBytes) -> () {
    unsafe { drop(value.into_box()) }
}

/// Fetches the next item from the pipeline, if one exists.
///
/// We use a callback model to ensure that the underlying buffer gets properly freed.
/// The callback will be called if any ONLY if an item is available.
/// The callback will be passed the raw buffer, which the caller must not store or use after the callback returns, because the driver will free it immediately after the callback returns.
///
/// This function will return immediately after freeing the buffer, which will happen after the callback returns.
#[no_mangle]
extern "C" fn pipeline_next_item_raw(pipeline: *mut Pipeline) -> FfiResult<FfiBytes> {
    let pipeline = ffi_try!(unsafe { pipeline.as_mut() }.ok_or(ResultCode::NullPointer));
    ffi_try!(pipeline.next_item_raw())
        .map(|b| FfiBytes::new(b))
        .into()
}

#[no_mangle]
extern "C" fn pipeline_enqueue_data_raw<'a>(
    pipeline: *mut Pipeline,
    partition_id: FfiSlice<'a>,
    buffer: FfiSlice<'a>,
) -> ResultCode {
    let pipeline = ffi_try!(unsafe { pipeline.as_mut() }.ok_or(ResultCode::NullPointer));
    let partition_id = ffi_try!(unsafe { partition_id.to_str() });
    let buffer = ffi_try!(unsafe { buffer.to_slice() });

    pipeline.enqueue_items_raw(partition_id, buffer).into()
}
