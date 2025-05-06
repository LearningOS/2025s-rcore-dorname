//! Implementation of syscalls
//!
//! The single entry point to all system calls, [`syscall()`], is called
//! whenever userspace wishes to perform a system call using the `ecall`
//! instruction. In this case, the processor raises an 'Environment call from
//! U-mode' exception, which is handled as one of the cases in
//! [`crate::trap::trap_handler`].
//!
//! For clarity, each single syscall is implemented as its own function, named
//! `sys_` then the name of the syscall. You can find functions like this in
//! submodules, and you should also implement syscalls this way.

/// write syscall
const SYSCALL_WRITE: usize = 64;
/// exit syscall
const SYSCALL_EXIT: usize = 93;
/// yield syscall
const SYSCALL_YIELD: usize = 124;
/// gettime syscall
const SYSCALL_GET_TIME: usize = 169;
/// trace syscall
const SYSCALL_TRACE: usize = 410;
/// mmap syscall
const SYSCALL_MMAP: usize = 222;
/// munmap syscall
const SYSCALL_MUNMAP: usize = 215;
/// sbrk syscall
const SYSCALL_SBRK: usize = 214;

const SYS_CALL_KEYS: [usize; 6] = [SYSCALL_WRITE, SYSCALL_EXIT, SYSCALL_YIELD, SYSCALL_GET_TIME, SYSCALL_TRACE, SYSCALL_SBRK];

mod fs;
mod process;

use alloc::vec::Vec;
use fs::*;
use process::*;
use crate::task::{get_current_task, increase_syscall_times};

/// 定义单个系统调用统计结构体
#[derive(Debug, Clone)]
struct SingleSyscallTimes {
    id: usize,
    times: usize,
}

/// 定义系统调用递增函数
impl SingleSyscallTimes {
    fn increase(&mut self) {
        self.times += 1;
    }
}

/// 首先定义一个新的结构体来包装系统调用统计
#[derive(Debug, Clone)]
pub struct SyscallStats {
    /// 任务id
    pub task_id: usize,
    syscall_times: Vec<SingleSyscallTimes>,
}

impl SyscallStats {
    /// 初始化系统调用统计
    pub fn new(task_id: usize) -> Self {
        SyscallStats {
            task_id,
            syscall_times: SYS_CALL_KEYS.iter().map(|&id| SingleSyscallTimes { id, times: 0 }).collect() 
        }
    }
    /// 递增系统调用次数
    pub fn increase(&mut self, id: usize) {
        if let Some(stat) = self.syscall_times.iter_mut().find(|item| item.id == id) {
            stat.increase();
        }
    }

    /// 根据系统调用id 获取系统调用次数
    pub fn get_syscall_times(&self, id: usize) -> usize {
        self.syscall_times.iter().find(|&item| item.id == id).unwrap().times
    }
}

/// handle syscall exception with `syscall_id` and other arguments
pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    let task_id = get_current_task();
    match syscall_id {
        SYSCALL_WRITE => {
            increase_syscall_times(task_id, SYSCALL_WRITE);
            sys_write(args[0], args[1] as *const u8, args[2])
        },
        SYSCALL_EXIT => {
            increase_syscall_times(task_id, SYSCALL_EXIT);
            sys_exit(args[0] as i32)
        },
        SYSCALL_YIELD => {
            increase_syscall_times(task_id, SYSCALL_YIELD);
            sys_yield()
        },
        SYSCALL_GET_TIME => {
            increase_syscall_times(task_id, SYSCALL_GET_TIME);
            sys_get_time(args[0] as *mut TimeVal, args[1])
        },
        SYSCALL_TRACE => {
            increase_syscall_times(task_id, SYSCALL_TRACE);
            sys_trace(args[0], args[1], args[2])
        },
        SYSCALL_MMAP => {
            increase_syscall_times(task_id, SYSCALL_MMAP);
            sys_mmap(args[0], args[1], args[2])
        },
        SYSCALL_MUNMAP => {
            increase_syscall_times(task_id, SYSCALL_MUNMAP);
            sys_munmap(args[0], args[1])
        },
        SYSCALL_SBRK => {
            increase_syscall_times(task_id, SYSCALL_SBRK);
            sys_sbrk(args[0] as i32)
        },
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}