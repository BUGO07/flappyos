/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use alloc::{collections::vec_deque::VecDeque, vec::Vec};
use bevy_ecs::prelude::*;
use pc_keyboard::{KeyCode, Keyboard, ScancodeSet1, layouts::Us104Key};

use crate::WORLD;

#[derive(Resource)]
pub struct KeyboardState {
    pub keyboard: Keyboard<Us104Key, ScancodeSet1>,
    pub scancodes: VecDeque<u8>,
    pub keys_down: Vec<KeyCode>,
}

pub fn keyboard_interrupt_handler(_stack_frame: *mut crate::ints::StackFrame) {
    unsafe {
        let mut keyboard_state = WORLD
            .get_mut()
            .unwrap()
            .get_resource_mut::<KeyboardState>()
            .unwrap();
        keyboard_state
            .scancodes
            .push_back(crate::utils::asm::inb(0x60))
    };
    crate::ints::pic::send_eoi(1);
}

// * i can probably do this inside keyboard_interrupt_handler with &mut World
pub fn keyboard_system(mut keyboard_state: ResMut<KeyboardState>) {
    if !keyboard_state.scancodes.is_empty() {
        let scancode = keyboard_state.scancodes.pop_front().unwrap();
        if let Ok(Some(key_event)) = keyboard_state.keyboard.add_byte(scancode) {
            if key_event.state == pc_keyboard::KeyState::Down {
                if !keyboard_state.keys_down.contains(&key_event.code) {
                    keyboard_state.keys_down.push(key_event.code);
                }
            } else {
                keyboard_state.keys_down.retain(|&x| x != key_event.code);
            }
        }
    }
}
