/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::arch::asm;

#[repr(C, packed)]
#[derive(Default, Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct StackFrame {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rbp: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rbx: u64,
    pub rax: u64,
    pub vector: u64,
    pub ec: u64,
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

#[repr(C, packed)]
pub struct IdtPtr {
    limit: u16,
    base: u64,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct IdtEntry {
    offset0: u16,
    selector: u16,
    ist: u8,
    typeattr: u8,
    offset1: u16,
    offset2: u32,
    zero: u32,
}

impl IdtEntry {
    const fn new() -> Self {
        Self {
            offset0: 0,
            selector: 0,
            ist: 0,
            typeattr: 0,
            offset1: 0,
            offset2: 0,
            zero: 0,
        }
    }

    fn set(&mut self, isr: u64, typeattr: Option<u8>, ist: Option<u8>) {
        self.typeattr = typeattr.unwrap_or(0x8E);
        self.ist = ist.unwrap_or(0);

        let addr = isr;
        self.offset0 = (addr & 0xFFFF) as u16;
        self.offset1 = ((addr >> 16) & 0xFFFF) as u16;
        self.offset2 = (addr >> 32) as u32;

        unsafe {
            asm!("mov {0:x}, cs", out(reg) self.selector, options(nomem, nostack, preserves_flags));
        }
    }
}

core::arch::global_asm! {
    r#"
.extern isr_handler
isr_common_stub:
    cld

    push rax
    push rbx
    push rcx
    push rdx
    push rsi
    push rdi
    push rbp
    push r8
    push r9
    push r10
    push r11
    push r12
    push r13
    push r14
    push r15

    mov rdi, rsp
    call isr_handler

    pop r15
    pop r14
    pop r13
    pop r12
    pop r11
    pop r10
    pop r9
    pop r8
    pop rbp
    pop rdi
    pop rsi
    pop rdx
    pop rcx
    pop rbx
    pop rax

    add rsp, 16

    iretq

.macro isr number
    isr_\number:
.if !(\number == 8 || (\number >= 10 && \number <= 14) || \number == 17 || \number == 21 || \number == 29 || \number == 30)
    push 0
.endif
    push \number
    jmp isr_common_stub
.endm

.altmacro
.macro isr_insert number
    .section .text
    isr \number

    .section .data
    .quad isr_\number
.endm

.section .data
.byte 1
.align 8
isr_table:
.set i, 0
.rept 256
    isr_insert %i
    .set i, i + 1
.endr
.global isr_table
    "#
}

type HandlerFn = fn(frame: *mut StackFrame);
static mut HANDLERS: [Option<HandlerFn>; 256] = [None; 256];
static mut IDT: [IdtEntry; 256] = [IdtEntry::new(); 256];
static mut IDTR: IdtPtr = IdtPtr {
    limit: (size_of::<IdtEntry>() * 256 - 1) as u16,
    base: 0,
};

const EXCEPTION_NAMES: [&str; 32] = [
    "divide by zero",
    "debug",
    "non-maskable interrupt",
    "breakpoint",
    "detected overflow",
    "out-of-bounds",
    "invalid opcode",
    "no coprocessor",
    "double fault",
    "coprocessor segment overrun",
    "bad TSS",
    "segment not present",
    "stack fault",
    "general protection fault",
    "page fault",
    "unknown interrupt",
    "coprocessor fault",
    "alignment check",
    "machine check",
    "reserved",
    "reserved",
    "reserved",
    "reserved",
    "reserved",
    "reserved",
    "reserved",
    "reserved",
    "reserved",
    "reserved",
    "reserved",
    "reserved",
    "reserved",
];

#[unsafe(no_mangle)]
extern "C" fn isr_handler(regs: *mut StackFrame) {
    unsafe {
        let registers = &*regs;

        if registers.vector == 14 {
            let cr2: u64;
            asm!("mov {}, cr2", out(reg) cr2, options(nomem, nostack, preserves_flags));
            panic!("page fault at {:#x}, registers:\n\n{:?}", cr2, registers);
        }

        if registers.vector < 32 {
            panic!(
                "exception: {}, registers:\n\n{:?}",
                EXCEPTION_NAMES[registers.vector as usize], registers
            );
        }

        if let Some(handler) = HANDLERS[registers.vector as usize] {
            handler(regs);
        }
    };
}

unsafe extern "C" {
    static isr_table: u64;
}

pub fn init() {
    let table = &raw const isr_table;
    unsafe {
        for (i, entry) in IDT.iter_mut().enumerate() {
            entry.set(
                *table.add(i),
                if i == 0x80 { Some(0xEE) } else { Some(0x8E) },
                None,
            );
        }
        IDTR.base = IDT.as_ptr() as u64;

        asm!("cli; lidt [{}]", in(reg) &IDTR, options(readonly, nostack, preserves_flags));

        // HANDLERS[0x80] = Some(crate::arch::system::syscall::syscall_dispatch); // skibidi syscall handler

        HANDLERS[0x20] = Some(crate::time::pit::timer_interrupt_handler);
        HANDLERS[0x21] = Some(crate::keyboard::keyboard_interrupt_handler);
    }
}

pub fn install_interrupt(vector: u8, func: HandlerFn) {
    unsafe {
        HANDLERS[vector as usize] = Some(func);
    }
}

pub fn clear_interrupt(vector: u8) {
    unsafe {
        HANDLERS[vector as usize] = None;
    }
}

pub mod pic {
    /*
        Copyright (C) 2025 bugo07
        Released under EUPL 1.2 License
    */

    use crate::{
        info,
        utils::asm::{inb, outb},
    };

    const PIC_EOI: u8 = 0x20;
    const ICW1_ICW4: u8 = 0x01;
    const ICW4_8086: u8 = 0x01;
    const ICW1_INIT: u8 = 0x10;
    const PIC1_COMMAND: u16 = 0x20;
    const PIC2_COMMAND: u16 = 0xA0;
    const PIC1_DATA: u16 = 0x21;
    const PIC2_DATA: u16 = 0xA1;

    pub fn send_eoi(irq: u8) {
        if irq >= 8 {
            outb(PIC2_COMMAND, PIC_EOI);
        }
        outb(PIC1_COMMAND, PIC_EOI);
    }

    pub fn interrupts_enabled() -> bool {
        let rflags: u64;
        unsafe {
            core::arch::asm!("pushfq; pop {}", out(reg) rflags);
        }
        (rflags & (1 << 9)) != 0
    }

    pub fn unmask_all() {
        outb(PIC1_DATA, 0);
        outb(PIC2_DATA, 0);
    }

    pub fn mask_all() {
        outb(PIC1_DATA, 0xff);
        outb(PIC2_DATA, 0xff);
    }

    pub fn mask(mut irq: u8) {
        let port: u16;
        if irq < 8 {
            port = PIC1_DATA;
        } else {
            port = PIC2_DATA;
            irq -= 8;
        }
        outb(port, inb(port) | (1 << irq));
        // debug!("masked irq {}", irq);
    }

    pub fn unmask(mut irq: u8) {
        let port: u16;
        if irq < 8 {
            port = PIC1_DATA;
        } else {
            port = PIC2_DATA;
            irq -= 8;
        }
        outb(port, inb(port) & !(1 << irq));
        // debug!("unmasked irq {}", irq);
    }

    pub fn init() {
        info!("remapping...");

        let i1 = inb(PIC1_DATA);
        let i2 = inb(PIC2_DATA);

        outb(PIC1_COMMAND, ICW1_INIT | ICW1_ICW4);
        outb(PIC2_COMMAND, ICW1_INIT | ICW1_ICW4);

        outb(PIC1_DATA, 0x20);
        outb(PIC2_DATA, 0x28);

        outb(PIC1_DATA, 0x04);
        outb(PIC2_DATA, 0x02);

        outb(PIC1_DATA, ICW4_8086);
        outb(PIC2_DATA, ICW4_8086);

        outb(PIC1_DATA, i1);
        outb(PIC2_DATA, i2);
    }
}
