//! Process management syscalls

use core::mem::size_of;

use crate::{
    mm::PTEFlags,
    task::{
        add_address_mapping, change_program_brk, cur_user_addr_translate,
        cur_user_addr_translate_page_align_checked, exit_current_and_run_next, get_syscall_counter,
        remove_address_mapping, suspend_current_and_run_next,
    },
    timer::get_time_us,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    let va = ts as *const _ as usize;
    let len = size_of::<TimeVal>();
    if let Some(pa) =
        cur_user_addr_translate_page_align_checked(va, len, Some(PTEFlags::R | PTEFlags::W))
    {
        let pa = pa as *mut _;
        // Safety: We can directly dereference the user's physical address,
        // bacause the user's address area is identically mapped in the kerenl's page table.
        unsafe {
            *pa = TimeVal {
                sec: us / 1_000_000,
                usec: us % 1_000_000,
            };
        }
        0
    } else {
        -1
    }
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    match trace_request {
        0 => {
            let va = id as *const u8 as usize;
            if let Some(pa) = cur_user_addr_translate(va, Some(PTEFlags::R)) {
                let pa = pa as *const u8;
                // Safety: We can directly dereference the user's physical address,
                // bacause the user's address area is identically mapped in the kerenl's page table.
                unsafe { *pa as isize }
            } else {
                -1
            }
        }
        1 => {
            let va = id as *const u8 as usize;
            if let Some(pa) = cur_user_addr_translate(va, Some(PTEFlags::R | PTEFlags::W)) {
                let pa = pa as *mut u8;
                // Safety: We can directly dereference the user's physical address,
                // bacause the user's address area is identically mapped in the kerenl's page table.
                unsafe { *pa = data as u8 };
                0
            } else {
                -1
            }
        }
        2 => {
            let syscall_id = id;
            get_syscall_counter(syscall_id) as isize
        }
        _ => -1,
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!("kernel: sys_mmap");
    let start_va = start;
    let end_va = start + len;
    if add_address_mapping(start_va, end_va, port) {
        0
    } else {
        -1
    }
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap");
    let start_va = start;
    let end_va = start + len;
    if remove_address_mapping(start_va, end_va) {
        0
    } else {
        -1
    }
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
