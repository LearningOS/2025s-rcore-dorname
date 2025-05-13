//! Process management syscalls
use crate::config::PAGE_SIZE;
use crate::task::{change_program_brk, exit_current_and_run_next, get_current_task, get_syscall_times, suspend_current_and_run_next};
use crate::task::current_user_token;
use crate::mm::{frame_alloc, PageTable, VirtPageNum, PTEFlags};
use crate::timer::get_time_us;

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
    let page_table = PageTable::from_token(token);
    //4、检查虚拟地址是否在当前用户空间可写
    let vpn = VirtPageNum::from(ts_va / PAGE_SIZE);
    if let Some(pte) = page_table.translate(vpn) {
        // 5、检查页表项是否具有用户权限和写权限
        if pte.readable() && pte.writable() {
            //6、将时间写入TimeVal结构体中
            let us:usize = get_time_us();
            let sec:usize = us / 1_000_000;
            let usec:usize = us % 1_000_000;
            unsafe {
                (*ts).set_time(sec, usec);
            }
        }
    }
    0
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    //1、获取当前用户的页表 token
    let token = current_user_token();
    //2、获取虚拟地址
    let trace_request_va = trace_request as usize;
    //3、根据用户token读取页表
    let page_table = PageTable::from_token(token);
    
    //4、检查虚拟地址是否在当前用户空间可写
    let vpn = VirtPageNum::from(trace_request_va / PAGE_SIZE);
    if let Some(pte) = page_table.translate(vpn) {
        // 5、检查页表项是否具有用户权限和写权限
        if pte.readable() && pte.writable() {
            //6、根据trace_request的值进行不同的操作
            match trace_request {
                0 => {
                    //7、读取任务id地址处的值
                    // 检查id地址是否有效
                    let id_vpn = VirtPageNum::from(id / PAGE_SIZE);
                    if let Some(id_pte) = page_table.translate(id_vpn) {
                        if id_pte.readable() {
                            let addr = unsafe {
                                *(id as *const u8)
                            };
                            return addr as isize;
                        }
                    }
                    return -1;
                },
                1 => {
                    //8、写入任务id地址处的值
                    // 检查id地址是否有效
                    let id_vpn = VirtPageNum::from(id / PAGE_SIZE);
                    if let Some(id_pte) = page_table.translate(id_vpn) {
                        if id_pte.writable() {
                            unsafe {
                                *(id as *mut u8) = data as u8
                            };
                            return 0;
                        }
                    }
                    return -1;
                },
                2 => {
                    return get_syscall_times(get_current_task(), id) as isize;
                }
                _ => {
                    return -1;
                }
            }
        }
    }
    -1
}

/// 系统调用：mmap
pub fn sys_mmap(start: usize, len: usize, prot: usize) -> isize {
    trace!("kernel: sys_mmap");
    fn prot_to_pte_flags(prot: usize) -> Option<PTEFlags> {
        // 检查 prot 是否合法
        if (prot & !0x7) != 0 {
            return None;
        }
        if (prot & 0x7) == 0 {
            return None;
        }
    
        // 构建页表项标志
        let mut flags = PTEFlags::V | PTEFlags::U;  // 有效位和用户态访问位
        if (prot & 0x1) != 0 {
            flags |= PTEFlags::R;  // 可读
        }
        if (prot & 0x2) != 0 {
            flags |= PTEFlags::W;  // 可写
        }
        if (prot & 0x4) != 0 {
            flags |= PTEFlags::X;  // 可执行
        }
        Some(flags)
    }
    
    // 1. 检查参数合法性
    if start % PAGE_SIZE != 0 {
        return -1;  // start 没有按页对齐
    }
    
    // 2. 检查 prot 参数
    let flags = match prot_to_pte_flags(prot) {
        Some(f) => f,
        None => return -1,
    };
    
    // 3. 如果 len 为 0，直接返回成功
    if len == 0 {
        return 0;
    }
    
    // 4. 计算需要的页数
    let page_count = (len + PAGE_SIZE - 1) / PAGE_SIZE;
    
    // 5. 获取当前任务的页表
    let token = current_user_token();
    let mut page_table = PageTable::from_token(token);
    
    // 6. 检查目标虚存区间是否已被映射
    for i in 0..page_count {
        let vpn = VirtPageNum::from(start / PAGE_SIZE + i);
        if page_table.translate(vpn).is_some() {
            return -1;  // 页面已被映射
        }
    }
    
    // 7. 分配物理页并建立映射
    for i in 0..page_count {
        let vpn = VirtPageNum::from(start / PAGE_SIZE + i);
        if let Some(frame) = frame_alloc() {
            page_table.map(vpn, frame.ppn, flags);
        } else {
            // 物理内存不足，需要回滚已分配的页
            for j in 0..i {
                let vpn = VirtPageNum::from(start / PAGE_SIZE + j);
                page_table.unmap(vpn);
            }
            return -1;
        }
    }
    
    0  // 成功返回0
}

/// 系统调用：munmap
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap");
    
    // 1. 检查参数合法性
    if start % PAGE_SIZE != 0 {
        return -1;  // start 没有按页对齐
    }
    
    // 2. 如果 len 为 0，直接返回成功
    if len == 0 {
        return 0;
    }
    
    // 3. 计算页数
    let page_count = (len + PAGE_SIZE - 1) / PAGE_SIZE;
    
    // 4. 获取当前任务的页表
    let token = current_user_token();
    let mut page_table = PageTable::from_token(token);
    
    // 5. 检查所有页面是否都已映射
    for i in 0..page_count {
        let vpn = VirtPageNum::from(start / PAGE_SIZE + i);
        if page_table.translate(vpn).is_none() {
            return -1;  // 存在未映射的页面
        }
    }
    
    // 6. 解除映射  
    for i in 0..page_count {
        let vpn = VirtPageNum::from(start / PAGE_SIZE + i);
        page_table.unmap(vpn);
    }
    
    0  // 成功返回0
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

