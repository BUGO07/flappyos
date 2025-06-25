/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::fmt::Write;

use super::asm::{inb, outb};

const COM1_BASE: u16 = 0x3F8;

const COM1_DATA: u16 = COM1_BASE;

const COM1_INTERRUPT_ENABLE: u16 = COM1_BASE + 1;

const COM1_LINE_CONTROL: u16 = COM1_BASE + 3;

const COM1_FIFO_CONTROL: u16 = COM1_BASE + 2;

const COM1_MODEM_CONTROL: u16 = COM1_BASE + 4;

const COM1_LINE_STATUS: u16 = COM1_BASE + 5;

pub fn init() {
    outb(COM1_INTERRUPT_ENABLE, 0x00);
    outb(COM1_LINE_CONTROL, 0x80);
    outb(COM1_DATA, 0x03);
    outb(COM1_INTERRUPT_ENABLE, 0x00);
    outb(COM1_LINE_CONTROL, 0x03);
    outb(COM1_FIFO_CONTROL, 0xC7);
    outb(COM1_MODEM_CONTROL, 0x0B);
}

pub fn serial_write(byte: u8) {
    while (inb(COM1_LINE_STATUS) & 0x20) == 0 {}
    outb(COM1_DATA, byte);
}

pub fn serial_read() -> u8 {
    while (inb(COM1_LINE_STATUS) & 1) == 0 {}
    inb(COM1_DATA)
}

pub struct SerialWriter;

impl Write for SerialWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        if s != "\x1b[6n" {
            for byte in s.bytes() {
                serial_write(byte);
            }
        }
        Ok(())
    }
}

#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments) {
    write!(SerialWriter, "{args}").ok();
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::utils::serial::_print(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($fmt:expr) => ($crate::print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::print!(
        concat!($fmt, "\n"), $($arg)*));
}

pub mod color {
    use alloc::{format, string::String};

    pub const RESET: &str = "\x1b[0m";

    pub const BLACK: &str = "\x1b[38;5;16m";
    pub const GRAY: &str = "\x1b[38;5;243m";
    pub const DARK_RED: &str = "\x1b[38;5;88m";
    pub const RED: &str = "\x1b[38;5;1m";
    pub const LIGHT_RED: &str = "\x1b[38;5;9m";
    pub const GREEN: &str = "\x1b[38;5;46m";
    pub const DARK_GREEN: &str = "\x1b[38;5;34m";
    pub const LIGHT_GREEN: &str = "\x1b[38;5;10m";
    pub const YELLOW: &str = "\x1b[38;5;226m";
    pub const BLUE: &str = "\x1b[38;5;69m";
    pub const PURPLE: &str = "\x1b[38;5;91m";
    pub const PINK: &str = "\x1b[38;5;207m";
    pub const CYAN: &str = "\x1b[38;5;51m";
    pub const WHITE: &str = "\x1b[38;5;7m";
    pub const WHITE_BRIGHT: &str = "\x1b[38;5;15m";

    pub const BLACK_BG: &str = "\x1b[48;5;16m";
    pub const GRAY_BG: &str = "\x1b[48;5;243m";
    pub const DARK_RED_BG: &str = "\x1b[48;5;88m";
    pub const RED_BG: &str = "\x1b[48;5;1m";
    pub const LIGHT_RED_BG: &str = "\x1b[48;5;9m";
    pub const GREEN_BG: &str = "\x1b[48;5;46m";
    pub const DARK_GREEN_BG: &str = "\x1b[48;5;34m";
    pub const LIGHT_GREEN_BG: &str = "\x1b[48;5;10m";
    pub const YELLOW_BG: &str = "\x1b[48;5;226m";
    pub const BLUE_BG: &str = "\x1b[48;5;69m";
    pub const PURPLE_BG: &str = "\x1b[48;5;91m";
    pub const PINK_BG: &str = "\x1b[48;5;207m";
    pub const CYAN_BG: &str = "\x1b[48;5;51m";
    pub const WHITE_BG: &str = "\x1b[48;5;7m";
    pub const WHITE_BRIGHT_BG: &str = "\x1b[48;5;15m";

    pub fn rgb(r: u8, g: u8, b: u8, bg: bool) -> String {
        let first = if bg { "48" } else { "38" };
        format!("\x1b[{first};2;{r};{g};{b}m") // super dim
    }
}

pub fn log_message(level: &str, color: &str, mut module_path: &str, args: core::fmt::Arguments) {
    // #[cfg(not(any(feature = "tests", feature = "uacpi_test")))]
    {
        if level == "dbug" && !cfg!(debug_assertions) {
            return;
        }
        module_path = module_path.split("::").last().unwrap();
        if module_path == "x86_64" || module_path == "aarch64" {
            module_path = "chronos";
        }

        let digits = 5;

        let elapsed_ns = crate::time::preferred_timer_ns();
        let subsecond_ns = elapsed_ns % 1_000_000_000;

        let divisor = 10u64.pow(9 - digits);
        let subsecond = subsecond_ns / divisor;

        let elapsed_ms = elapsed_ns / 1_000_000;
        let seconds_total = elapsed_ms / 1000;
        let seconds = seconds_total % 60;
        let minutes_total = seconds_total / 60;
        let minutes = minutes_total % 60;
        let hours = minutes_total / 60;

        println!(
            "[{:02}:{:02}:{:02}.{:0width$}] [ {}{}{} ] {}{}:{} {}",
            hours,
            minutes,
            seconds,
            subsecond,
            color,
            level,
            color::RESET,
            color::GRAY,
            module_path,
            color::RESET,
            args,
            width = digits as usize
        );
    }
}

#[macro_export]
macro_rules! ok {
    ($($arg:tt)*) => {
        $crate::utils::serial::log_message(
            " OK ",
            $crate::utils::serial::color::DARK_GREEN,
            module_path!(),
            format_args!($($arg)*),
        )
    };
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => ($crate::utils::serial::log_message("info", $crate::utils::serial::color::GREEN, module_path!(), format_args!($($arg)*)));
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => ($crate::utils::serial::log_message("dbug", $crate::utils::serial::color::CYAN, module_path!(), format_args!($($arg)*)));
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => ($crate::utils::serial::log_message("warn", $crate::utils::serial::color::YELLOW, module_path!(), format_args!($($arg)*)));
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => ($crate::utils::serial::log_message("error", $crate::utils::serial::color::RED, module_path!(), format_args!($($arg)*)));
}
