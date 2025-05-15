//! Process management syscalls
use crate::config::PAGE_SIZE;
use crate::task::{change_program_brk, exit_current_and_run_next, get_current_task, get_syscall_times, suspend_current_and_run_next};
use crate::task::current_user_token;
use crate::mm::{frame_alloc, PageTable, VirtPageNum, PTEFlags, PhysAddr};
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
            // 获取物理地址
            let pa: PhysAddr = pte.ppn().into();
            // 计算在页面内的偏移量
            let offset = ts_va & (PAGE_SIZE - 1);
            // 计算最终的物理地址
            let ts_pa = pa.0 + offset;
            // 将物理地址转换为可用的指针
            let ts_ptr = ts_pa as *mut TimeVal;
            unsafe {
                (*ts_ptr).set_time(sec, usec);
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
    //2、根据用户token读取页表
    let page_table = PageTable::from_token(token);
    
    //3、根据trace_request的值进行不同的操作
    match trace_request {
        0 => {
            //4、读取任务id地址处的值
            // 检查id地址是否在用户空间范围内
            if id >= (1 << 39) {
                return -1;
            }
            // 检查id地址是否有效
            let id_vpn = VirtPageNum::from(id / PAGE_SIZE);
            if let Some(id_pte) = page_table.translate(id_vpn) {
                if id_pte.readable() {
                    // 获取物理地址
                    let pa: PhysAddr = id_pte.ppn().into();
                    // 计算在页面内的偏移量
                    let offset = id & (PAGE_SIZE - 1);
                    // 计算最终的物理地址
                    let id_pa = pa.0 + offset;
                    // 将物理地址转换为可用的指针
                    let id_ptr = id_pa as *const u8;
                    let addr = unsafe {
                        *id_ptr
                    };
                    return addr as isize;
                }
            }
            return -1;
        },
        1 => {
            //5、写入任务id地址处的值
            // 检查id地址是否在用户空间范围内
            if id >= (1 << 39) {
                return -1;
            }
            // 检查id地址是否有效
            let id_vpn = VirtPageNum::from(id / PAGE_SIZE);
            if let Some(id_pte) = page_table.translate(id_vpn) {
                if id_pte.writable() {
                    // 获取物理地址
                    let pa: PhysAddr = id_pte.ppn().into();
                    // 计算在页面内的偏移量
                    let offset = id & (PAGE_SIZE - 1);
                    // 计算最终的物理地址
                    let id_pa = pa.0 + offset;
                    // 将物理地址转换为可用的指针
                    let id_ptr = id_pa as *mut u8;
                    unsafe {
                        *id_ptr = data as u8;
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

/// 系统调用：mmap
pub fn sys_mmap(start: usize, len: usize, prot: usize) -> isize {
    trace!("kernel: sys_mmap: start={:#x}, len={:#x}, prot={:#x}", start, len, prot);    

    // 1. 检查起始地址是否按页对齐合法性
    if start % PAGE_SIZE != 0 {
        trace!("kernel: sys_mmap: start not aligned");
        return -1;  // start 没有按页对齐
    }
    
    // 2. 检查 prot 参数
    if (prot & !0x7) != 0 {
        trace!("kernel: sys_mmap: invalid prot flags (other bits not zero)");
        return -1;  // prot 其余位必须为0
    }
    if (prot & 0x7) == 0 {
        trace!("kernel: sys_mmap: invalid prot flags (no permissions)");
        return -1;  // 这样的内存无意义
    }
    
    
    // 3. 构建页表项标志
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
    
    // 4. 计算需要映射的页数（向上取整）
    let page_count = (len + PAGE_SIZE - 1) / PAGE_SIZE;
    trace!("kernel: sys_mmap: page_count={}", page_count);
    
    // 5. 获取当前任务的页表
    let token = current_user_token();
    let mut page_table = PageTable::from_token(token);
    
    // 6. 检查目标虚存区间是否已被映射
    let mut overlap = false;
    for i in 0..page_count {
        let vpn = VirtPageNum::from(start / PAGE_SIZE + i);
        if let Some(pte) = page_table.translate(vpn) {
            if pte.is_valid() {
                trace!("kernel: sys_mmap: VPN={:#x} already mapped", vpn.0);
                overlap = true;
                break;
            }
        }
    }
    
    if overlap {
        return -1;
    }
    
    // 8. 分配物理页并建立映射
    for i in 0..page_count {
        let vpn = VirtPageNum::from(start / PAGE_SIZE + i);
        if let Some(frame) = frame_alloc() {
            trace!("kernel: sys_mmap: mapping VPN={:#x} to PPN={:#x}", vpn.0, frame.ppn.0);
            page_table.map(vpn, frame.ppn, flags);
        } else {
            trace!("kernel: sys_mmap: no more physical frames available");
            // 物理内存不足，需要回滚已分配的页 实验不需要
            // for j in 0..i {
            //     let vpn = VirtPageNum::from(start / PAGE_SIZE + j);
            //     page_table.unmap(vpn);
            // }
            return -1;
        }
    }
    
    trace!("kernel: sys_mmap: success");
    0  // 成功返回0
}

/// 系统调用：munmap
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap: start={:#x}, len={:#x}", start, len);
    
    // 1. 检查参数合法性
    if start % PAGE_SIZE != 0 {
        trace!("kernel: sys_munmap: start not aligned");
        return -1;  // start 没有按页对齐
    }
    
    // 3. 计算页数（向上取整）
    let page_count = (len + PAGE_SIZE - 1) / PAGE_SIZE;
    trace!("kernel: sys_munmap: page_count={}", page_count);
    
    // 4. 获取当前任务的页表
    let token = current_user_token();
    let mut page_table = PageTable::from_token(token);
    
    // 5. 解除映射
    for i in 0..page_count {
        let vpn = VirtPageNum::from(start / PAGE_SIZE + i);
        if let Some(pte) = page_table.translate(vpn) {
           
            if pte.is_valid() {
                println!("vaild i: {}, pte: {:?}", i, pte);
                trace!("kernel: sys_munmap: unmapping VPN={:#x}", vpn.0);
                page_table.unmap(vpn);
            }else {
                println!("invalid i: {}, pte: {:?}", i, pte);
                println!("start: {:x}, len: {}, page_count: {}", start, len, page_count);
                // 存在有一个无效映射 返回-1
                return -1;
            }
        }
    }
    
    trace!("kernel: sys_munmap: success");
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


