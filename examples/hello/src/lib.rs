#[unsafe(no_mangle)]
pub extern "C" fn alloc(size: i32) -> i32 {
    let mut buf = vec![0; size as usize];
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr as i32
}

#[unsafe(no_mangle)]
pub extern "C" fn handler(ptr: i32, len: i32) -> i32 {
    let _input = unsafe { std::slice::from_raw_parts(ptr as *const u8, len as usize) };

    let response = b"{\"status\":200,\"body\":\"hello\"}";

    let total_len = 4 + response.len();
    let out_ptr = alloc(total_len as i32);

    unsafe {
        let len_bytes = (response.len() as u32).to_le_bytes();

        std::ptr::copy_nonoverlapping(len_bytes.as_ptr(), out_ptr as *mut u8, 4);

        std::ptr::copy_nonoverlapping(response.as_ptr(), (out_ptr + 4) as *mut u8, response.len());
    }

    out_ptr
}
