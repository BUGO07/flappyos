/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::sync::atomic::Ordering;

use crate::{arch::time::preferred_timer_ms, info, utils::asm::_rdtsc};

use super::{Timer, TimerKind, register_timer};

pub fn measure_cpu_frequency() -> u64 {
    if super::kvm::supported() {
        return super::kvm::tsc_freq();
    }

    let mut cpu_freq_hz = 0;

    for _ in 0..3 {
        let start_cycles = _rdtsc();
        let start_ticks = preferred_timer_ms();

        super::busywait_ms(50);

        let end_cycles = _rdtsc();
        let end_ticks = preferred_timer_ms();

        let elapsed_ticks = end_ticks - start_ticks;
        let elapsed_cycles = end_cycles - start_cycles;

        let cycles_per_tick = elapsed_cycles / elapsed_ticks;

        cpu_freq_hz += cycles_per_tick * 1000;
    }

    cpu_freq_hz / 3
}

pub fn init() {
    info!("setting up...");
    let freq = measure_cpu_frequency();
    register_timer(Timer::new(
        TimerKind::TSC,
        _rdtsc(),
        freq,
        true,
        10,
        |timer: &Timer| {
            (((_rdtsc() - timer.start) as u128 * 1_000_000_000) / timer.frequency as u128) as u64
        },
        0,
    ));
    crate::CPU_FREQ.store(freq, Ordering::Relaxed);
    info!("done");
}
