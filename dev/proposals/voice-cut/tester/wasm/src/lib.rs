//! `plan(ptr, len)`: a transcript as JSON in linear memory, with the judge's `answers`
//! beside it once there are some (and, listening for more `cues`, the `meant` let in) →
//! `[u32 length][JSON]` holding `{edit_list, keep, edited_duration, questions, intents,
//! cues}`, as `shroom-cut` prints. The page calls `alloc` for the
//! input and `free` for both buffers.

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
    let read = serde_json::from_slice::<serde_json::Value>(input).and_then(|v| {
        let get = |k: &str| v.get(k).cloned().unwrap_or(serde_json::Value::Null);
        let answers: Vec<shroom_edit::Answer> = if get("answers").is_null() { Vec::new() } else { serde_json::from_value(get("answers"))? };
        let meant: Vec<shroom_edit::Meant> = if get("meant").is_null() { Vec::new() } else { serde_json::from_value(get("meant"))? };
        let cues: shroom_edit::CueWords = if get("cues").is_null() { Default::default() } else { serde_json::from_value(get("cues"))? };
        Ok((serde_json::from_value::<shroom_edit::Transcript>(v)?, answers, meant, cues))
    });
    let out = match read {
        Ok((t, answers, meant, cues)) => {
            let o = shroom_edit::Options { cues, ..shroom_edit::Options::default() };
            let list = shroom_edit::plan_meant(&t, o, &meant, &answers);
            let heard: Vec<serde_json::Value> = shroom_edit::find_cues_in(&t.words, cues)
                .iter()
                .map(|c| serde_json::json!({ "first": c.first, "last": c.last, "start": c.start, "end": c.end, "said": c.said }))
                .collect();
            serde_json::json!({ "edit_list": list, "keep": list.keep(), "edited_duration": list.edited_duration(),
                "questions": shroom_edit::questions_meant(&t, o, &meant), "intents": shroom_edit::intents(&t, o), "cues": heard })
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
