/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::{
    arch::asm,
    ffi::{c_int, c_void},
};

#[inline(always)]
pub fn halt() {
    unsafe { asm!("hlt") };
}

#[inline(always)]
pub fn halt_loop() -> ! {
    loop {
        halt();
    }
}

#[inline(always)]
pub fn halt_no_ints() {
    toggle_ints(false);
    halt();
}

#[inline(always)]
pub fn halt_with_ints() {
    toggle_ints(true);
    halt();
}

#[inline(always)]
pub fn toggle_ints(val: bool) {
    unsafe {
        if val {
            asm!("sti");
        } else {
            asm!("cli");
        }
    }
}

#[inline(always)]
pub fn int_status() -> bool {
    let r: u64;
    unsafe { asm!("pushfq; pop {}", out(reg) r) };
    (r & (1 << 9)) != 0
}

#[inline(always)]
pub fn without_ints<F, R>(closure: F) -> R
where
    F: FnOnce() -> R,
{
    let enabled = int_status();
    if enabled {
        toggle_ints(false);
    }
    let ret = closure();
    if enabled {
        toggle_ints(true);
    }
    ret
}

pub struct CpuidResult {
    pub eax: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,
}

#[inline(always)]
pub fn _cpuid_count(leaf: u32, sub_leaf: u32) -> CpuidResult {
    let eax;
    let ebx;
    let ecx;
    let edx;

    unsafe {
        asm!(
            "mov {0:r}, rbx",
            "cpuid",
            "xchg {0:r}, rbx",
            out(reg) ebx,
            inout("eax") leaf => eax,
            inout("ecx") sub_leaf => ecx,
            out("edx") edx,
            options(nostack, preserves_flags),
        );
    }
    CpuidResult { eax, ebx, ecx, edx }
}

#[inline(always)]
pub fn _cpuid(leaf: u32) -> CpuidResult {
    _cpuid_count(leaf, 0)
}

#[inline(always)]
pub fn _rdtsc() -> u64 {
    let low: u32;
    let high: u32;
    unsafe {
        asm!(
            "rdtsc",
            out("eax") low,
            out("edx") high,
            options(nomem, nostack, preserves_flags),
        );
    }
    ((high as u64) << 32) | (low as u64)
}

#[inline(always)]
pub fn rdmsr(msr: u32) -> u64 {
    let high: u32;
    let low: u32;
    unsafe {
        asm!(
            "rdmsr",
            in("ecx") msr,
            out("edx") high,
            out("eax") low,
        );
    }
    (high as u64) << 32 | low as u64
}

#[inline(always)]
pub fn wrmsr(msr: u32, value: u64) {
    let high = (value >> 32) as u32;
    let low = value as u32;
    unsafe {
        asm!(
            "wrmsr",
            in("ecx") msr,
            in("edx") high,
            in("eax") low,
        );
    }
}

pub fn kvm_base() -> u32 {
    if in_hypervisor() {
        let mut signature: [u32; 3] = [0; 3];
        for base in (0x40000000..0x40010000).step_by(0x100) {
            let id = _cpuid(base);

            signature[0] = id.ebx;
            signature[1] = id.ecx;
            signature[2] = id.edx;

            let mut output: [u8; 12] = [0; 12];

            for (i, num) in signature.iter().enumerate() {
                let bytes = num.to_le_bytes();
                output[i * 4..(i + 1) * 4].copy_from_slice(&bytes);
            }
            if memcmp(
                c"KVMKVMKVM".as_ptr() as *const c_void,
                output.as_ptr() as *const c_void,
                12,
            ) != 0
            {
                return base;
            }
        }
    }
    0
}

pub fn in_hypervisor() -> bool {
    let id = _cpuid(1);

    (id.ecx & (1 << 31)) != 0
}

#[unsafe(no_mangle)]
pub extern "C" fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void {
    unsafe {
        asm!(
            "rep movsb",
            inout("rdi") dest => _,
            inout("rsi") src => _,
            inout("rcx") n => _,
            options(nostack, preserves_flags)
        );
        dest
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn memset(dest: *mut c_void, val: c_int, n: usize) -> *mut c_void {
    unsafe {
        asm!(
            "rep stosb",
            inout("rdi") dest => _,
            in("al") val as u8,
            inout("rcx") n => _,
            options(nostack, preserves_flags)
        );
        dest
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void {
    unsafe {
        let d = dest as usize;
        let s = src as usize;
        if d < s || d >= (s + n) {
            memcpy(dest, src, n)
        } else {
            asm!(
                "std",
                "add rsi, rcx",
                "add rdi, rcx",
                "dec rsi",
                "dec rdi",
                "rep movsb",
                "cld",
                inout("rdi") dest => _,
                inout("rsi") src => _,
                inout("rcx") n => _,
                options(nostack, preserves_flags)
            );
            dest
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int {
    let mut i = 0;
    while i < n {
        let va = unsafe { *(a.add(i) as *const u8) };
        let vb = unsafe { *(b.add(i) as *const u8) };
        if va != vb {
            return va as c_int - vb as c_int;
        }
        i += 1;
    }
    0
}

#[inline(always)]
pub fn mmio_read(addr: u64, width: usize) -> u64 {
    {
        match width {
            1 => {
                let value: u8;
                unsafe {
                    asm!(
                        "mov {0}, byte ptr [{1:r}]",
                        out(reg_byte) value,
                        in(reg) addr,
                        options(nostack, readonly)
                    );
                }
                value as u64
            }
            2 => {
                let value: u16;
                unsafe {
                    asm!(
                        "mov {0:x}, word ptr [{1:r}]",
                        out(reg) value,
                        in(reg) addr,
                        options(nostack, readonly)
                    );
                }
                value as u64
            }
            4 => {
                let value: u32;
                unsafe {
                    asm!(
                        "mov {0:e}, dword ptr [{1:r}]",
                        out(reg) value,
                        in(reg) addr,
                        options(nostack, readonly)
                    );
                }
                value as u64
            }
            8 => {
                let value: u64;
                unsafe {
                    asm!(
                        "mov {0:r}, qword ptr [{1:r}]",
                        out(reg) value,
                        in(reg) addr,
                        options(nostack, readonly)
                    );
                }
                value
            }
            _ => panic!("mmio::read: invalid width {width}"),
        }
    }
}

#[inline(always)]
pub fn mmio_write(addr: u64, val: u64, width: usize) {
    match width {
        1 => {
            let val = val as u8;
            unsafe {
                asm!(
                    "mov byte ptr [{0:r}], {1}",
                    in(reg) addr,
                    in(reg_byte) val,
                    options(nostack)
                );
            }
        }
        2 => {
            let val = val as u16;
            unsafe {
                asm!(
                    "mov word ptr [{0:r}], {1:x}",
                    in(reg) addr,
                    in(reg) val,
                    options(nostack)
                );
            }
        }
        4 => {
            let val = val as u32;
            unsafe {
                asm!(
                    "mov dword ptr [{0:r}], {1:e}",
                    in(reg) addr,
                    in(reg) val,
                    options(nostack)
                );
            }
        }
        8 => unsafe {
            asm!(
                "mov qword ptr [{0:r}], {1:r}",
                in(reg) addr,
                in(reg) val,
                options(nostack)
            );
        },
        _ => panic!("mmio::write: invalid width {width}"),
    }
}

#[inline(always)]
pub fn inb(port: u16) -> u8 {
    let value: u8;
    unsafe {
        asm!("in al, dx", out("al") value, in("dx") port, options(nomem, nostack, preserves_flags));
    }
    value
}

#[inline(always)]
pub fn inw(port: u16) -> u16 {
    let value: u16;
    unsafe {
        asm!("in ax, dx", out("ax") value, in("dx") port, options(nomem, nostack, preserves_flags));
    }
    value
}

#[inline(always)]
pub fn inl(port: u16) -> u32 {
    let value: u32;
    unsafe {
        asm!("in eax, dx", out("eax") value, in("dx") port, options(nomem, nostack, preserves_flags));
    }
    value
}

#[inline(always)]
pub fn outb(port: u16, value: u8) {
    unsafe {
        asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack, preserves_flags));
    }
}

#[inline(always)]
pub fn outw(port: u16, value: u16) {
    unsafe {
        asm!("out dx, ax", in("dx") port, in("ax") value, options(nomem, nostack, preserves_flags));
    }
}

#[inline(always)]
pub fn outl(port: u16, value: u32) {
    unsafe {
        asm!("out dx, eax", in("dx") port, in("eax") value, options(nomem, nostack, preserves_flags));
    }
}
