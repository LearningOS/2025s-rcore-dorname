//! Process management syscalls
use crate::task::{change_program_brk, exit_current_and_run_next, suspend_current_and_run_next};
use crate::task::current_user_token;
use crate::mm::{PageTable, VirtAddr};
use crate::timer::{get_time, get_time_us};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

impl TimeVal {
    fn set_time(&mut self, sec: usize, usec: usize) {
        self.sec = sec;
        self.usec = usec;
    }
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
/// 获取当前系统时间，并写入TimeVal结构体中
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    //1、获取当前用户的页表 token
    let token = current_user_token();
    //2、获取虚拟地址
    let ts_va = ts as usize;
    //3、根据用户token读取页表
    let pgae_table = PageTable::from_token(token);
    //4、检查虚拟地址是否在当前用户空间可写
    if let Some(pte)  = pgae_table.translate(VirtAddr::from(ts_va).into())  {
        // 5、检查页表项是否具有用户权限和写权限
        if pte.readable() && pte.writable() {
            //6、将时间写入TimeVal结构体中
            unsafe {
                (*ts).set_time(get_time(), get_time_us());
            }
            return 0;
        }
    }
    -1
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(_trace_request: usize, _id: usize, _data: usize) -> isize {
    trace!("kernel: sys_trace");
    -1
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    -1
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    -1
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
