//! File and filesystem-related syscalls
use crate::fs::{
    create_entry, delete_entry, find_file, open_file, File, OSInode, OpenFlags, Stat, StatMode,
};
use crate::mm::{translated_byte_buffer, translated_str, translated_write, UserBuffer};
use crate::task::{current_task, current_user_token};

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_write", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.writable() {
            return -1;
        }
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        let x = file.write(UserBuffer::new(
            translated_byte_buffer(token, buf, len).unwrap(),
        ));
        // info!("write fd {}, , x {}", fd, x);
        x as isize
    } else {
        // info!("not write fd {}, len {}", fd, inner.fd_table.len());
        -1
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_read", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        // info!(">= fd {}, len{}", fd, inner.fd_table.len());
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.readable() {
            // info!("wtf, fd {}, len {}", fd, len);
            return -1;
        }
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        trace!("kernel: sys_read .. file.read");
        let x = file.read(UserBuffer::new(
            translated_byte_buffer(token, buf, len).unwrap(),
        ));
        // info!("fd {}, read len {}", fd, x);
        x as isize
    } else {
        // info!("not find fd {}, len {}", fd, inner.fd_table.len());
        -1
    }
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    trace!("kernel:pid[{}] sys_open", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        let mut inner = task.inner_exclusive_access();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    trace!("kernel:pid[{}] sys_close", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}

/// YOUR JOB: Implement fstat.
pub fn sys_fstat(fd: usize, st: *mut Stat) -> isize {
    trace!("kernel:pid[{}] sys_fstat", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.as_ref();
        let osinode = unsafe { &*(file as *const dyn File as *const OSInode) };
        let inode = osinode.get_inode();
        let ino = inode.get_inode_id() as u64;
        let mode: StatMode = if inode.is_dir() {
            StatMode::DIR
        } else {
            StatMode::FILE
        };
        let nlink = inode.get_link_count() as u32;
        let src = Stat::new(0, ino, mode, nlink);
        translated_write(token, st, src);
        0
    } else {
        -1
    }
}

/// YOUR JOB: Implement linkat.
pub fn sys_linkat(old_name: *const u8, new_name: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_linkat", current_task().unwrap().pid.0);
    let token = current_user_token();
    let old_name = translated_str(token, old_name);
    let new_name = translated_str(token, new_name);
    if old_name == new_name {
        return -1;
    }
    if let Some(inode) = find_file(&old_name) {
        if create_entry(&new_name, inode) {
            0
        } else {
            -1
        }
    } else {
        -1
    }
}

/// YOUR JOB: Implement unlinkat.
pub fn sys_unlinkat(name: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_unlinkat NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    let token = current_user_token();
    let name = translated_str(token, name);
    if let Some(inode) = find_file(&name) {
        if delete_entry(&name, inode) {
            0
        } else {
            -1
        }
    } else {
        -1
    }
}
