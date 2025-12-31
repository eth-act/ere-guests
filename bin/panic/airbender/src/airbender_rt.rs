use core::ptr::addr_of_mut;

core::arch::global_asm!(include_str!("./asm_reduced.S"));

unsafe extern "C" {
    // Boundaries of the heap
    static mut _sheap: usize;
    static mut _eheap: usize;

    // Boundaries of the .data section (and it's part in ROM)
    static mut _sidata: usize;
    static mut _sdata: usize;
    static mut _edata: usize;

    // Boundaries of the .rodata section
    static mut _sirodata: usize;
    static mut _srodata: usize;
    static mut _erodata: usize;
}

unsafe fn load_to_ram(src: *const u8, dst_start: *mut u8, dst_end: *mut u8) {
    let offset = dst_end.addr() - dst_start.addr();

    unsafe { core::ptr::copy_nonoverlapping(src, dst_start, offset) };
}

#[unsafe(link_section = ".init.rust")]
#[unsafe(export_name = "_start_rust")]
unsafe extern "C" fn start_rust() -> ! {
    unsafe {
        load_to_ram(
            addr_of_mut!(_sirodata) as *const u8,
            addr_of_mut!(_srodata) as *mut u8,
            addr_of_mut!(_erodata) as *mut u8,
        );
        load_to_ram(
            addr_of_mut!(_sidata) as *const u8,
            addr_of_mut!(_sdata) as *mut u8,
            addr_of_mut!(_edata) as *mut u8,
        );
    };

    crate::main();

    unsafe { core::hint::unreachable_unchecked() }
}
