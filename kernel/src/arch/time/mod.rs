/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

// pub mod hpet;
pub mod kvm;
pub mod pit;
pub mod tsc;

use alloc::string::String;

use crate::{info, utils::heapless::HeaplessVec};

pub static mut TIMERS: HeaplessVec<Timer, 10> = HeaplessVec::new();

pub fn init() {
    pit::init();
    kvm::init();
    tsc::init();
}

pub struct Timer {
    pub kind: TimerKind,
    pub start: u64,
    pub frequency: u64,
    pub supported: bool,
    pub priority: u8,
    pub elapsed_ns: fn(&Self) -> u64,
    pub offset: u64,
}

impl Timer {
    pub fn new(
        kind: TimerKind,
        start: u64,
        frequency: u64,
        supported: bool,
        priority: u8,
        elapsed_ns: fn(&Self) -> u64,
        offset: u64,
    ) -> Self {
        Self {
            kind,
            start,
            frequency,
            supported,
            priority,
            elapsed_ns,
            offset,
        }
    }
    pub fn kind(&self) -> &TimerKind {
        &self.kind
    }
    pub fn name(&self) -> &'static str {
        self.kind.into()
    }
    pub fn is_supported(&self) -> bool {
        self.supported
    }
    pub fn priority(&self) -> u8 {
        self.priority
    } // unused as of now
    pub fn get_offset(&self) -> u64 {
        self.offset
    }
    pub fn set_offset(&mut self, offset: u64) {
        self.offset = offset;
    }
    pub fn elapsed(&self) -> u64 {
        (self.elapsed_ns)(self)
    }
    pub fn elapsed_pretty(&self, digits: u32) -> alloc::string::String {
        elapsed_time_pretty((self.elapsed_ns)(self), digits)
    }
}

pub fn register_timer(timer: Timer) {
    info!("registering timer - {} [{}]", timer.name(), timer.priority);
    unsafe { TIMERS.push(timer).ok() };
    get_timers().sort_by(|a, b| a.priority.cmp(&b.priority));
}

pub fn get_timers() -> &'static mut HeaplessVec<Timer, 10> {
    unsafe { &mut TIMERS }
}

pub fn get_timer(kind: &TimerKind) -> &mut Timer {
    get_timers().iter_mut().find(|x| x.kind() == kind).unwrap()
}

pub fn get_timer_by_name(name: &str) -> &mut Timer {
    get_timers().iter_mut().find(|x| x.name() == name).unwrap()
}

#[inline(always)]
pub fn preferred_timer_ms() -> u64 {
    preferred_timer_ns() / 1_000_000
}

pub fn preferred_timer_ns() -> u64 {
    for timer in get_timers().iter() {
        if timer.is_supported() {
            return (timer.elapsed_ns)(timer);
        }
    }

    0
}

#[inline(always)]
pub fn preferred_timer_pretty(digits: u32) -> String {
    elapsed_time_pretty(preferred_timer_ns(), digits)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerKind {
    KVM,
    TSC,
    HPET,
    PIT,
}

impl From<TimerKind> for &'static str {
    fn from(val: TimerKind) -> Self {
        match val {
            TimerKind::KVM => "kvm",
            TimerKind::TSC => "tsc",
            TimerKind::HPET => "hpet",
            TimerKind::PIT => "pit",
        }
    }
}

impl From<&'static str> for TimerKind {
    fn from(val: &'static str) -> Self {
        match val {
            "kvm" => TimerKind::KVM,
            "tsc" => TimerKind::TSC,
            "hpet" => TimerKind::HPET,
            "pit" => TimerKind::PIT,
            _ => TimerKind::PIT,
        }
    }
}

impl From<TimerKind> for u64 {
    fn from(val: TimerKind) -> Self {
        match val {
            TimerKind::KVM => 0,
            TimerKind::TSC => 1,
            TimerKind::HPET => 2,
            TimerKind::PIT => 3,
        }
    }
}

impl From<u64> for TimerKind {
    fn from(val: u64) -> Self {
        match val {
            0 => TimerKind::KVM,
            1 => TimerKind::TSC,
            2 => TimerKind::HPET,
            3 => TimerKind::PIT,
            _ => TimerKind::PIT,
        }
    }
}

pub fn elapsed_time_pretty(ns: u64, digits: u32) -> alloc::string::String {
    let subsecond_ns = ns % 1_000_000_000;

    let divisor = 10u64.pow(9 - digits);
    let subsecond = subsecond_ns / divisor;

    let elapsed_ms = ns / 1_000_000;
    let seconds_total = elapsed_ms / 1000;
    let seconds = seconds_total % 60;
    let minutes_total = seconds_total / 60;
    let minutes = minutes_total % 60;
    let hours = minutes_total / 60;

    alloc::format!(
        "{:02}:{:02}:{:02}.{:0width$}",
        hours,
        minutes,
        seconds,
        subsecond,
        width = digits as usize
    )
}

#[inline(always)]
pub fn busywait_ns(ns: u64) {
    let start = preferred_timer_ns();
    while preferred_timer_ns() - start < ns {
        core::hint::spin_loop();
    }
}

#[inline(always)]
pub fn busywait_ms(ms: u64) {
    busywait_ns(ms * 1_000_000);
}
