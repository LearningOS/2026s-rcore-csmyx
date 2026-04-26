//! Types related to task management
use alloc::vec;
use alloc::vec::Vec;

use super::TaskContext;
use crate::config::TRAP_CONTEXT_BASE;
use crate::mm::{
    kernel_stack_position, MapPermission, MemorySet, PTEFlags, PhysPageNum, VirtAddr, KERNEL_SPACE,
};
use crate::syscall::{SyscallId, SYSCALL_IDS};
use crate::trap::{trap_handler, TrapContext};

/// The task control block (TCB) of a task.
pub struct TaskControlBlock {
    /// Save task context
    pub task_cx: TaskContext,

    /// Maintain the execution status of the current process
    pub task_status: TaskStatus,

    /// Application address space
    pub memory_set: MemorySet,

    /// The phys page number of trap context
    pub trap_cx_ppn: PhysPageNum,

    /// The size(top addr) of program which is loaded from elf file
    pub base_size: usize,

    /// Heap bottom
    pub heap_bottom: usize,

    /// Program break
    pub program_brk: usize,

    /// syscall counter, (syscall_id, count)
    pub syscall_counter: Vec<(SyscallId, usize)>,
}

impl TaskControlBlock {
    /// get the trap context
    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.get_mut()
    }
    /// Remove a framed area from current task's page table.
    pub fn user_addr_remove_framed_area(&mut self, start_va: usize, end_va: usize) -> bool {
        self.memory_set
            .try_remove_framed_area(start_va.into(), end_va.into())
    }
    /// Insert a framed area into current task's page table.
    pub fn user_addr_insert_framed_area(
        &mut self,
        start_va: usize,
        end_va: usize,
        permission: usize,
    ) -> bool {
        if permission >> 3 != 0 {
            return false;
        }
        let mut perm = MapPermission::U;
        if permission & 0x1 != 0 {
            perm |= MapPermission::R;
        }
        if permission & 0x2 != 0 {
            perm |= MapPermission::W;
        }
        if permission & 0x4 != 0 {
            perm |= MapPermission::X;
        }

        self.memory_set
            .try_insert_framed_area(start_va.into(), end_va.into(), perm)
    }
    /// Translate virtual address to physical address by current memory set.
    /// return None if is not page aligned or the mapping is not valid
    pub fn user_addr_translate_page_align_checked(
        &self,
        va: usize,
        len: usize,
        flags: Option<PTEFlags>,
    ) -> Option<usize> {
        self.memory_set
            .translate_addr_page_align_checked(va.into(), (va + len - 1).into(), flags)
            .map(|pa| pa.0)
    }
    /// Translate virtual address to physical address by current memory set.
    /// Return None if the mapping is not valid.
    pub fn user_addr_translate(&self, va: usize, flags: Option<PTEFlags>) -> Option<usize> {
        self.memory_set
            .translate_addr(va.into(), flags)
            .map(|pa| pa.0)
    }
    /// get the user token
    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }
    /// Based on the elf info in program, build the contents of task in a new address space
    pub fn new(elf_data: &[u8], app_id: usize) -> Self {
        // memory_set with elf program headers/trampoline/trap context/user stack
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT_BASE).into())
            .unwrap()
            .ppn();
        let task_status = TaskStatus::Ready;
        // map a kernel-stack in kernel space
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(app_id);
        KERNEL_SPACE.exclusive_access().insert_framed_area(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            MapPermission::R | MapPermission::W,
        );
        let mut syscall_counter = vec![];
        for id in SYSCALL_IDS {
            syscall_counter.push((id, 0));
        }
        let task_control_block = Self {
            task_status,
            task_cx: TaskContext::goto_trap_return(kernel_stack_top),
            memory_set,
            trap_cx_ppn,
            base_size: user_sp,
            heap_bottom: user_sp,
            program_brk: user_sp,
            syscall_counter,
        };
        // prepare TrapContext in user space
        let trap_cx = task_control_block.get_trap_cx();
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_top,
            trap_handler as usize,
        );
        task_control_block
    }
    /// change the location of the program break. return None if failed.
    pub fn change_program_brk(&mut self, size: i32) -> Option<usize> {
        let old_break = self.program_brk;
        let new_brk = self.program_brk as isize + size as isize;
        if new_brk < self.heap_bottom as isize {
            return None;
        }
        let result = if size < 0 {
            self.memory_set
                .shrink_to(VirtAddr(self.heap_bottom), VirtAddr(new_brk as usize))
        } else {
            self.memory_set
                .append_to(VirtAddr(self.heap_bottom), VirtAddr(new_brk as usize))
        };
        if result {
            self.program_brk = new_brk as usize;
            Some(old_break)
        } else {
            None
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
/// task status: UnInit, Ready, Running, Exited
pub enum TaskStatus {
    /// uninitialized
    UnInit,
    /// ready to run
    Ready,
    /// running
    Running,
    /// exited
    Exited,
}
