use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Mutex;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeTag {
    Int = 0,
    Bool = 1,
    String = 2,
    Nil = 3,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NativeValue {
    pub tag: NativeTag,
    pub int_value: i64,
    pub bool_value: u8,
    pub string_ptr: *const c_char,
}

impl NativeValue {
    pub fn nil() -> Self {
        Self {
            tag: NativeTag::Nil,
            int_value: 0,
            bool_value: 0,
            string_ptr: std::ptr::null(),
        }
    }

    pub fn int(v: i64) -> Self {
        Self {
            tag: NativeTag::Int,
            int_value: v,
            bool_value: 0,
            string_ptr: std::ptr::null(),
        }
    }

    pub fn bool(v: bool) -> Self {
        Self {
            tag: NativeTag::Bool,
            int_value: 0,
            bool_value: if v { 1 } else { 0 },
            string_ptr: std::ptr::null(),
        }
    }

    pub fn string(s: &str) -> Self {
        let c = CString::new(s).unwrap_or_else(|_| CString::new("").expect("empty cstring"));
        let ptr = c.into_raw();
        Self {
            tag: NativeTag::String,
            int_value: 0,
            bool_value: 0,
            string_ptr: ptr,
        }
    }
}

pub type NativeCallback = extern "C" fn(
    args: *const NativeValue,
    arg_count: usize,
    out_result: *mut NativeValue,
) -> i32;

static REGISTRY: Lazy<Mutex<HashMap<String, NativeCallback>>> = Lazy::new(|| Mutex::new(HashMap::new()));

#[no_mangle]
pub unsafe extern "C" fn swiftvm_register_native(
    selector: *const c_char,
    callback: NativeCallback,
) -> i32 {
    if selector.is_null() {
        return -1;
    }

    let selector = match CStr::from_ptr(selector).to_str() {
        Ok(v) => v.to_string(),
        Err(_) => return -2,
    };

    let mut registry = match REGISTRY.lock() {
        Ok(v) => v,
        Err(_) => return -3,
    };
    registry.insert(selector, callback);
    0
}

#[no_mangle]
pub unsafe extern "C" fn swiftvm_call_native(
    selector: *const c_char,
    args: *const NativeValue,
    arg_count: usize,
    out_result: *mut NativeValue,
) -> i32 {
    if selector.is_null() || out_result.is_null() {
        return -1;
    }

    let selector = match CStr::from_ptr(selector).to_str() {
        Ok(v) => v.to_string(),
        Err(_) => return -2,
    };

    let callback = {
        let registry = match REGISTRY.lock() {
            Ok(v) => v,
            Err(_) => return -3,
        };
        match registry.get(&selector) {
            Some(cb) => *cb,
            None => return -4,
        }
    };

    callback(args, arg_count, out_result)
}

#[no_mangle]
pub unsafe extern "C" fn swiftvm_string_free(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    let _ = CString::from_raw(ptr);
}

pub fn register_native_rust(selector: &str, callback: NativeCallback) -> Result<(), i32> {
    let selector_c = CString::new(selector).map_err(|_| -2)?;
    let rc = unsafe { swiftvm_register_native(selector_c.as_ptr(), callback) };
    if rc == 0 {
        Ok(())
    } else {
        Err(rc)
    }
}

pub fn call_native_rust(selector: &str, args: &[NativeValue]) -> Result<NativeValue, i32> {
    let selector_c = CString::new(selector).map_err(|_| -2)?;
    let mut out = NativeValue::nil();
    let rc = unsafe {
        swiftvm_call_native(
            selector_c.as_ptr(),
            args.as_ptr(),
            args.len(),
            &mut out as *mut NativeValue,
        )
    };
    if rc == 0 { Ok(out) } else { Err(rc) }
}

#[cfg(test)]
mod tests {
    use super::*;

    extern "C" fn sum2(args: *const NativeValue, arg_count: usize, out_result: *mut NativeValue) -> i32 {
        if args.is_null() || out_result.is_null() || arg_count < 2 {
            return -9;
        }

        let slice = unsafe { std::slice::from_raw_parts(args, arg_count) };
        let a = slice[0].int_value;
        let b = slice[1].int_value;

        unsafe {
            *out_result = NativeValue::int(a + b);
        }
        0
    }

    #[test]
    fn registers_and_calls_native_callback() {
        let selector = CString::new("math.sum2").unwrap();

        let rc = unsafe { swiftvm_register_native(selector.as_ptr(), sum2) };
        assert_eq!(rc, 0);

        let args = [NativeValue::int(20), NativeValue::int(22)];
        let mut out = NativeValue::nil();

        let rc = unsafe {
            swiftvm_call_native(
                selector.as_ptr(),
                args.as_ptr(),
                args.len(),
                &mut out as *mut NativeValue,
            )
        };

        assert_eq!(rc, 0);
        assert_eq!(out.tag, NativeTag::Int);
        assert_eq!(out.int_value, 42);
    }
}
