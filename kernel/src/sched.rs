/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

// ! i did NOT need to do ts

use core::{
    alloc::Layout,
    arch::asm,
    cell::OnceCell,
    ffi::c_void,
    sync::atomic::{AtomicU64, Ordering},
};

use alloc::{
    collections::{binary_heap::BinaryHeap, vec_deque::VecDeque},
    string::String,
    sync::{Arc, Weak},
    vec::Vec,
};

use crate::{
    asm::{halt_loop, memcpy},
    bootloader::get_hhdm_offset,
    ints::StackFrame,
    mem::{KERNEL_STACK_SIZE, PAGEMAP, Pagemap, USER_STACK_SIZE, flag, page_size},
    spinlock::SpinLock,
    time::preferred_timer_ns,
};

pub static mut SCHEDULER: OnceCell<Scheduler> = OnceCell::new();
pub static mut LAPIC_ARM: OnceCell<fn(ns: usize, vector: u8)> = OnceCell::new();
pub static mut NEXT_USTACK_ADDR: u64 = 0x0000_7FFF_FF00_0000;

fn next_pid() -> u64 {
    get_scheduler().next_pid.fetch_add(1, Ordering::Relaxed)
}

pub fn next_stack_address() -> u64 {
    unsafe {
        NEXT_USTACK_ADDR -= USER_STACK_SIZE as u64;
        NEXT_USTACK_ADDR
    }
}

pub struct Scheduler {
    pub processes: Vec<Arc<SpinLock<Process>>>,
    pub current: Option<Arc<SpinLock<Thread>>>,
    pub queue: BinaryHeap<Arc<SpinLock<Thread>>>,
    pub secondary_queue: VecDeque<Arc<SpinLock<Thread>>>,
    pub timeslice: usize,
    pub next_pid: AtomicU64,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self {
            processes: Vec::new(),
            current: None,
            queue: BinaryHeap::new(),
            secondary_queue: VecDeque::new(),
            timeslice: 3_000_000,
            next_pid: AtomicU64::new(0),
        }
    }
}

pub struct Process {
    pub name: &'static str,
    pub pid: u64,
    pub next_tid: AtomicU64,
    pub pagemap: Arc<SpinLock<Pagemap>>,
    pub children: Vec<Arc<SpinLock<Thread>>>,
}

unsafe impl Send for Process {}
unsafe impl Sync for Process {}

impl Process {
    pub fn new(name: &'static str) -> Self {
        let pid = next_pid();
        // debug!("spawning new process {}", _pid);
        Self {
            name,
            pid,
            next_tid: AtomicU64::new(1),
            pagemap: unsafe { PAGEMAP.get().unwrap().clone() },
            children: Vec::new(),
        }
    }

    pub fn get_name(&self) -> &'static str {
        self.name
    }

    pub fn get_pid(&self) -> u64 {
        self.pid
    }

    pub fn get_children(&self) -> &Vec<Arc<SpinLock<Thread>>> {
        &self.children
    }

    pub fn get_children_mut(&mut self) -> &mut Vec<Arc<SpinLock<Thread>>> {
        &mut self.children
    }

    pub fn next_tid(&mut self) -> u64 {
        self.next_tid.fetch_add(1, Ordering::Relaxed)
    }
}

pub fn schedule(regs: *mut StackFrame) {
    use core::{alloc::Layout, ffi::c_void};

    let mut next = None;

    if let Some(ct) = current_thread() {
        let mut t = ct.lock();
        memcpy(
            &raw mut t.regs as *mut c_void,
            regs as *mut c_void,
            size_of::<StackFrame>(),
        );
        if t.get_status() == &ThreadStatus::Running {
            t.set_status(ThreadStatus::Ready);
        }
        t.runtime += preferred_timer_ns() - t.schedule_time;
    }

    let scheduler = get_scheduler();

    let mut count = scheduler.queue.len();
    while count > 0 {
        if let Some(thread) = next_thread() {
            count -= 1;
            let mut t = thread.lock();
            match t.get_status() {
                ThreadStatus::Ready => {
                    next = Some(thread.clone());
                    break;
                }
                ThreadStatus::Sleeping(when) => {
                    if &preferred_timer_ns() >= when {
                        t.set_status(ThreadStatus::Ready);
                        next = Some(thread.clone());
                        break;
                    } else {
                        scheduler.secondary_queue.push_back(thread.clone());
                    }
                }
                ThreadStatus::Terminated => {
                    unsafe {
                        let parent = t.get_parent().upgrade().unwrap();
                        let mut p = parent.lock();

                        alloc::alloc::dealloc(
                            t.kstack as *mut u8,
                            Layout::from_size_align(KERNEL_STACK_SIZE, 0x8).unwrap(),
                        );

                        if t.ustack != 0 {
                            for i in (0..USER_STACK_SIZE).step_by(page_size::SMALL as usize) {
                                #[cfg(target_arch = "x86_64")]
                                p.pagemap
                                    .lock()
                                    .unmap(t.ustack + i as u64, page_size::SMALL);
                            }
                            alloc::alloc::dealloc(
                                (t.ustack_phys + get_hhdm_offset()) as *mut u8,
                                Layout::from_size_align(USER_STACK_SIZE, page_size::SMALL as usize)
                                    .unwrap(),
                            );
                        }

                        let tid = t.get_tid();
                        thread.force_unlock();
                        p.children.retain(|x| x.lock().get_tid() != tid);
                    };
                }
                ThreadStatus::Blocked => {
                    // ignored
                }
                _ => enqueue(thread.clone()),
            }
        }
    }

    for thread in scheduler.secondary_queue.drain(..) {
        match thread.lock().get_status() {
            ThreadStatus::Blocked => {
                get_scheduler().secondary_queue.push_back(thread.clone());
            }
            _ => enqueue(thread.clone()),
        }
    }

    if next.is_none() {
        next = Some(idle0().clone());
    }

    if let Some(ct) = current_thread() {
        enqueue(ct.clone());
    }

    scheduler.current = next.clone();

    let thread = next.unwrap();
    let mut t = thread.lock();

    t.set_status(ThreadStatus::Running);

    unsafe {
        // very weird ik
        core::arch::asm!("mov cr3, {}", in(reg) t.parent.upgrade().unwrap().lock().pagemap.lock().top_level as u64, options(nostack));
    }

    memcpy(
        regs as *mut c_void,
        &raw const t.regs as *const c_void,
        size_of::<StackFrame>(),
    );

    (unsafe { LAPIC_ARM.get().unwrap() })(scheduler.timeslice, 0xFF);
    t.schedule_time = preferred_timer_ns();
}

#[inline(always)]
pub fn enqueue(thread: Arc<SpinLock<Thread>>) {
    get_scheduler().queue.push(thread);
}

#[inline(always)]
pub fn next_thread() -> Option<Arc<SpinLock<Thread>>> {
    get_scheduler().queue.pop()
}

pub fn kill_process(pid: u64) -> bool {
    let scheduler = get_scheduler();
    if pid == 0 {
        // crate::drivers::acpi::shutdown();
    }
    if let Some(pos) = scheduler.processes.iter().position(|p| p.lock().pid == pid) {
        let proc = scheduler.processes.get(pos).unwrap();

        for thread in proc.lock().children.iter() {
            thread.lock().set_status(ThreadStatus::Terminated);
        }

        scheduler.processes.remove(pos);

        true
    } else {
        false
    }
}

pub fn spawn_process(name: &'static str) -> u64 {
    let scheduler = get_scheduler();
    let proc = Arc::new(SpinLock::new(Process::new(name)));
    scheduler.processes.push(proc.clone());
    proc.lock().get_pid()
}

pub fn init() {
    unsafe { SCHEDULER.set(Scheduler::default()).ok() };
    get_scheduler()
        .processes
        .push(Arc::new(SpinLock::new(Process::new("kernel"))));
}

pub fn start() -> ! {
    yld();
    halt_loop()
}

pub fn is_initialized() -> bool {
    unsafe { SCHEDULER.get().is_some() }
}

pub fn get_scheduler() -> &'static mut Scheduler {
    unsafe { SCHEDULER.get_mut().unwrap() }
}

pub fn get_proc_by_pid(pid: u64) -> Option<&'static Arc<SpinLock<Process>>> {
    get_scheduler().processes.iter().find(|p| {
        p.force_unlock();
        p.lock().pid == pid
    })
}

pub fn get_proc_by_name(name: &str) -> Option<&'static Arc<SpinLock<Process>>> {
    get_scheduler()
        .processes
        .iter()
        .find(|p| p.lock().name == name)
}

pub fn current_process() -> Option<Arc<SpinLock<Process>>> {
    get_scheduler()
        .current
        .as_ref()
        .map(|x| x.lock().get_parent().upgrade().unwrap())
}

#[derive(Debug, PartialEq, Eq)]
pub enum ThreadStatus {
    Ready,
    Running,
    Sleeping(u64), // ns
    Blocked,
    Terminated,
}

pub struct Thread {
    pub name: &'static str,
    pub tid: u64,
    pub args: Vec<String>,
    pub entry: u64,
    pub kstack: u64,
    pub ustack: u64,
    pub ustack_phys: u64,
    pub regs: StackFrame,
    pub parent: Weak<SpinLock<Process>>,
    pub status: ThreadStatus,
    pub runtime: u64,
    pub schedule_time: u64,
}

impl core::fmt::Debug for Thread {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Thread")
            .field("name", &self.name)
            .field("tid", &self.tid)
            .field("status", &self.status)
            .field("runtime", &self.runtime)
            .finish()
    }
}

impl PartialEq for Thread {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl PartialOrd for Thread {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for Thread {}

impl Ord for Thread {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        if self.runtime == 0 {
            return core::cmp::Ordering::Less;
        }
        other.runtime.cmp(&self.runtime)
    }
}

unsafe impl Send for Thread {}
unsafe impl Sync for Thread {}

impl Thread {
    pub fn new(
        proc: &'static Arc<SpinLock<Process>>,
        func: usize,
        name: &'static str,
        user: bool,
        args: Vec<String>,
    ) -> Self {
        Self::new_with_tid(proc, func, name, user, args, proc.lock().next_tid())
    }
    pub fn new_with_tid(
        proc: &'static Arc<SpinLock<Process>>,
        func: usize,
        name: &'static str,
        user: bool,
        args: Vec<String>,
        tid: u64,
    ) -> Self {
        let kstack = unsafe {
            alloc::alloc::alloc(Layout::from_size_align(KERNEL_STACK_SIZE, 0x8).unwrap()) as u64
        };

        let mut ustack: u64 = 0;
        let mut ustack_phys = 0;
        let mut argv_ptrs: [u64; 64] = [0; 64];
        let mut argc = 0;

        if user {
            ustack_phys = unsafe {
                alloc::alloc::alloc(
                    Layout::from_size_align(USER_STACK_SIZE, page_size::SMALL as usize).unwrap(),
                ) as u64
                    - get_hhdm_offset()
            };

            proc.force_unlock();

            ustack = next_stack_address();

            for i in (0..USER_STACK_SIZE).step_by(page_size::SMALL as usize) {
                proc.lock().pagemap.lock().map(
                    ustack + i as u64,
                    ustack_phys + i as u64,
                    flag::RW | flag::USER,
                    page_size::SMALL,
                );
            }

            let mut stack = [0u8; 0x1000];
            let mut string_data_offset = 0x100;

            for arg in &args {
                let bytes = arg.as_bytes();
                let len = bytes.len();

                if string_data_offset + len + 1 > stack.len() {
                    panic!("Not enough stack space for argv strings");
                }

                let str_start = string_data_offset;
                stack[str_start..str_start + len].copy_from_slice(bytes);
                stack[str_start + len] = 0;

                argv_ptrs[argc] = ustack + str_start as u64;
                string_data_offset += len + 1;
                argc += 1;
            }

            argv_ptrs[argc] = 0;
            argc += 1;

            for (i, &ptr) in argv_ptrs[..argc].iter().enumerate() {
                let bytes = ptr.to_le_bytes();
                stack[i * 8..(i + 1) * 8].copy_from_slice(&bytes);
            }

            memcpy(
                ustack as *mut c_void,
                stack.as_ptr() as *const c_void,
                stack.len(),
            );
        }

        Self {
            name,
            tid,
            kstack,
            ustack,
            ustack_phys,
            args,
            entry: func as u64,
            regs: StackFrame {
                #[cfg(target_arch = "x86_64")]
                cs: if user { 0x20 | 0x03 } else { 0x08 },
                #[cfg(target_arch = "x86_64")]
                ss: if user { 0x28 | 0x03 } else { 0x10 },
                #[cfg(target_arch = "x86_64")]
                rsp: if user {
                    (ustack + USER_STACK_SIZE as u64) & !0xF
                } else {
                    kstack + KERNEL_STACK_SIZE as u64
                },
                #[cfg(target_arch = "x86_64")]
                rip: func as u64,
                #[cfg(target_arch = "x86_64")]
                rsi: ustack,
                #[cfg(target_arch = "x86_64")]
                rdi: argc.saturating_sub(1) as u64,
                #[cfg(target_arch = "x86_64")]
                rflags: 0x202,

                // TODO: implement aarch64 properly
                #[cfg(target_arch = "aarch64")]
                sp: kstack + KERNEL_STACK_SIZE as u64,
                #[cfg(target_arch = "aarch64")]
                pc: func as u64,
                #[cfg(target_arch = "aarch64")]
                pstate: 0,

                #[cfg(target_arch = "riscv64")]
                sp: kstack + KERNEL_STACK_SIZE as u64,
                #[cfg(target_arch = "riscv64")]
                sepc: func as u64,
                ..Default::default()
            },
            parent: Arc::downgrade(proc),
            status: ThreadStatus::Ready,
            runtime: 0,
            schedule_time: 0,
        }
    }

    pub fn get_name(&self) -> &'static str {
        self.name
    }

    pub fn get_tid(&self) -> u64 {
        self.tid
    }

    pub fn get_parent(&self) -> &Weak<SpinLock<Process>> {
        &self.parent
    }

    pub fn get_status(&self) -> &ThreadStatus {
        &self.status
    }

    pub fn set_status(&mut self, status: ThreadStatus) {
        self.status = status;
    }

    pub fn is_user(&self) -> bool {
        self.ustack != 0
    }
}

pub fn spawn_thread(
    proc: &'static Arc<SpinLock<Process>>,
    func: usize,
    name: &'static str,
    user: bool,
) -> u64 {
    spawn_with_args(proc, func, name, user, Vec::new())
}

pub fn spawn_with_args(
    proc: &'static Arc<SpinLock<Process>>,
    func: usize,
    name: &'static str,
    user: bool,
    args: Vec<String>,
) -> u64 {
    proc.force_unlock();
    let thread = Arc::new(SpinLock::new(Thread::new(proc, func, name, user, args)));
    proc.force_unlock();
    proc.lock().get_children_mut().push(thread.clone());
    thread.force_unlock();
    let tid = thread.lock().get_tid();
    thread.force_unlock();
    enqueue(thread);
    tid
}

pub fn sleep(ns: u64) {
    if let Some(thread) = current_thread() {
        let mut t = thread.lock();
        t.set_status(ThreadStatus::Sleeping(preferred_timer_ns() + ns));
    }
    yld();
}

#[inline(always)]
pub fn sleep_ms(ms: u64) {
    sleep(ms * 1_000_000);
}

pub fn yld() {
    unsafe { asm!("int $0xFF") };
}

pub fn block() {
    if let Some(thread) = current_thread() {
        thread.lock().set_status(ThreadStatus::Blocked);
        yld();
    }
}

pub fn wake(thread: &Arc<SpinLock<Thread>>) {
    let mut t = thread.lock();
    if t.get_status() == &ThreadStatus::Blocked {
        t.set_status(ThreadStatus::Ready);
        enqueue(thread.clone());
    }
}

pub fn terminate() -> ! {
    if let Some(thread) = current_thread() {
        thread.lock().set_status(ThreadStatus::Terminated);
        yld();
    }
    halt_loop()
}

pub fn idle0() -> &'static Arc<SpinLock<Thread>> {
    static mut IDLE: OnceCell<Arc<SpinLock<Thread>>> = OnceCell::new();
    unsafe {
        IDLE.get_or_init(|| {
            let t = Arc::new(SpinLock::new(Thread::new_with_tid(
                crate::sched::get_proc_by_pid(0).unwrap(),
                halt_loop as usize,
                "idle",
                false,
                alloc::vec![],
                99,
            )));
            t.lock().set_status(ThreadStatus::Ready);
            t
        })
    }
}

pub fn current_thread() -> &'static mut Option<Arc<SpinLock<Thread>>> {
    &mut get_scheduler().current
}
