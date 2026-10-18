//! `plan(ptr, len)`: a transcript as JSON in linear memory → `[u32 length][JSON]` holding
//! `{edit_list, keep, edited_duration}`, as `shroom-cut` prints. The page calls `alloc`
//! for the input and `free` for both buffers.

use std::alloc::{Layout, alloc as raw_alloc, dealloc};

#[unsafe(no_mangle)]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    unsafe { raw_alloc(Layout::from_size_align(len.max(1), 1).unwrap()) }
}

#[unsafe(no_mangle)]
pub extern "C" fn free(ptr: *mut u8, len: usize) {
    unsafe { dealloc(ptr, Layout::from_size_align(len.max(1), 1).unwrap()) }
}

#[unsafe(no_mangle)]
pub extern "C" fn plan(ptr: *const u8, len: usize) -> *mut u8 {
    let input = unsafe { std::slice::from_raw_parts(ptr, len) };
    let out = match serde_json::from_slice::<shroom_edit::Transcript>(input) {
        Ok(t) => {
            let list = shroom_edit::plan(&t, shroom_edit::Options::default());
            serde_json::json!({ "edit_list": list, "keep": list.keep(), "edited_duration": list.edited_duration() })
        }
        Err(e) => serde_json::json!({ "error": e.to_string() }),
    };
    let body = out.to_string().into_bytes();
    let buf = alloc(body.len() + 4);
    unsafe {
        std::ptr::copy_nonoverlapping((body.len() as u32).to_le_bytes().as_ptr(), buf, 4);
        std::ptr::copy_nonoverlapping(body.as_ptr(), buf.add(4), body.len());
    }
    buf
}
