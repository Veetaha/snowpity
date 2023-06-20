use std::ffi::{c_void, CStr, CString};
use std::os::raw::c_char;
use crate::{Result, Error};

type FfiFunction = unsafe extern "C" fn(*mut c_char) -> OutPtr;

pub(crate) mod bindings {
    use super::*;

    extern "C" {
        pub(crate) fn Initialize(ffiCookies: *mut c_char) -> OutPtr;
        pub(crate) fn GetTweet(ffiTweetId: *mut c_char) -> OutPtr;
        pub(crate) fn free(ptr: *mut c_void);
    }
}

#[repr(C)]
pub(crate) struct OutPtr(*mut c_char);

impl Drop for OutPtr {
    fn drop(&mut self) {
        // SAFETY: We know our bindings use CGo's C string allocator, which uses C's malloc
        unsafe {
            bindings::free(self.0 as *mut c_void);
        }
    }
}

/// Same as [`ffi_call_raw_json`], but serializes the input to JSON first.
///
/// SAFETY: same as [`ffi_call_raw_json`]
pub(crate) unsafe fn call<Input, Output>(
    ffi_fn: FfiFunction,
    fn_name: &'static str,
    input: Input,
) -> Result<Output>
where
    Input: serde::Serialize,
    Output: serde::de::DeserializeOwned,
{
    let input = serde_json::to_string(&input).unwrap();
    call_raw_json(ffi_fn, fn_name, &input)
}

/// SAFETY: the given FFI function may have internal invariants that must be upheld
/// Since this code calls it, the caller must arrange for those invariants to be upheld
pub(crate) unsafe fn call_raw_json<Output>(
    ffi_fn: FfiFunction,
    fn_name: &'static str,
    input: &str,
) -> Result<Output>
where
    Output: serde::de::DeserializeOwned,
{
    let input = CString::new(input).unwrap();

    // SAFETY:
    // - We know the Go code will never modify the input chars, so we can remove constness
    // - The `ffi_fn` function's invariants must be upheld by the caller
    // - The `OutPtr` will free the memory when it is dropped
    let output: OutPtr = ffi_fn(input.as_ptr() as *mut c_char);

    if output.0.is_null() {
        return Err(Error::Fatal(format!(
            "Null pointer returned from the FFI function {fn_name}.\n\
            Input:\n{}",
            input.to_string_lossy(),
        )));
    }

    // SAFETY: we know CGo's CString properly null-terminates the string
    let output = CStr::from_ptr(output.0).to_str().map_err(|err| {
        Error::Fatal(format!(
            "Invalid UTF-8 returned from the FFI function {fn_name}.\n\
            Input:\n{}\n\
            Error:\n{err:#?}",
            input.to_string_lossy(),
        ))
    })?;

    // SAFETY: we use `DeserializeOwned`, which ensures nothing is borrwed
    // from the input.
    serde_json::from_str(output).map_err(|err| {
        Error::Fatal(format!(
            "Invalid JSON returned from the FFI function {fn_name}.\n\
            Input:\n{}\n\
            Output:\n{output}\n\
            Error:\n{err:#?}",
            input.to_string_lossy(),
        ))
    })
}
