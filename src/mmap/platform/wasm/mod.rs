use wasm_bindgen::prelude::*;
use js_sys::{Object, WebAssembly, Uint8Array};
use crate::mmap::error::MmapError;
extern crate alloc;
use alloc::vec;

pub(crate) unsafe fn wasm_mmap(
    addr: *mut u8,
    len: usize,
    _prot: u64,
    _flags: u64,
    _fd: u64,
    _offset: i64,
) -> Result<*mut u8, MmapError> {
    let pages = (len + 65535) / 65536;
    let descriptor = Object::new();
    
    js_sys::Reflect::set(
        &descriptor,
        &JsValue::from_str("initial"),
        &JsValue::from_f64(pages as f64),
    ).map_err(|_| MmapError {
        code: -1,
        message: "Failed to set memory descriptor".into(),
    })?;

    let memory = WebAssembly::Memory::new(&descriptor)
        .map_err(|_| MmapError {
            code: -1,
            message: "Failed to allocate WebAssembly memory".into(),
        })?;

    let array = Uint8Array::new(&memory.buffer());
    let ptr = if addr.is_null() {
        let buffer = vec![0u8; len].as_mut_ptr();
        array.raw_copy_to_ptr(buffer);
        buffer
    } else {
        array.raw_copy_to_ptr(addr);
        addr
    };

    if ptr.is_null() {
        return Err(MmapError {
            code: -1,
            message: "Failed to get WebAssembly memory pointer".into(),
        });
    }

    Ok(ptr)
}

pub(crate) unsafe fn wasm_munmap(addr: *mut u8, _len: usize) -> Result<(), MmapError> {
    // WebAssembly memory is managed by the runtime
    Ok(())
}

pub(crate) unsafe fn mremap(
    old_addr: *mut u8,
    old_size: usize,
    new_size: usize,
    _flags: u64,
) -> Result<*mut u8, MmapError> {
    let new_addr = wasm_mmap(
        core::ptr::null_mut(),
        new_size,
        0,
        0,
        0,
        0,
    )?;

    let copy_size = if new_size > old_size { old_size } else { new_size };
    core::ptr::copy_nonoverlapping(old_addr, new_addr, copy_size);

    wasm_munmap(old_addr, old_size)?;

    Ok(new_addr)
}
