//! Process management syscalls

use crate::{
    task::{exit_current_and_run_next, get_current_task, get_syscall_times, suspend_current_and_run_next},
    timer::get_time_us,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// get time with second and microsecond
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    unsafe {
        *ts = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    }
    0
}

// TODO: implement the syscall
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    // 获取当前任务
    match trace_request {
        0 => {
            // 读取任务id地址处的值
            let value = unsafe { *(id as *const u8) };
             value as isize
        }
        1 => {
            // 写入任务id地址处的值
            unsafe { *(id as *mut u8) = data as u8 };
             0
        }
        2 => {
            get_syscall_times(get_current_task(), id) as isize
        }
        _=> -1
    }
}
