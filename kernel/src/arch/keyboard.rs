/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use alloc::{collections::vec_deque::VecDeque, vec::Vec};
use bevy_ecs::prelude::*;
use pc_keyboard::{KeyCode, Keyboard, ScancodeSet1, layouts::Us104Key};

#[derive(Resource)]
pub struct KeyboardState {
    pub keyboard: Keyboard<Us104Key, ScancodeSet1>,
    pub scancodes: VecDeque<u8>,
    pub keys_down: Vec<KeyCode>,
    pub last_keys_down: Vec<KeyCode>,
}

impl KeyboardState {
    pub fn pressed(&self, key: KeyCode) -> bool {
        self.keys_down.contains(&key)
    }
    pub fn released(&self, key: KeyCode) -> bool {
        !self.keys_down.contains(&key)
    }
    pub fn pressed_any(&self) -> bool {
        !self.keys_down.is_empty()
    }
    pub fn just_pressed(&self, key: KeyCode) -> bool {
        self.keys_down.contains(&key) && !self.last_keys_down.contains(&key)
    }
    pub fn just_released(&self, key: KeyCode) -> bool {
        !self.keys_down.contains(&key) && self.last_keys_down.contains(&key)
    }
    pub fn just_pressed_any(&self) -> bool {
        self.keys_down.len() > self.last_keys_down.len()
    }
    pub fn just_released_any(&self) -> bool {
        self.keys_down.len() < self.last_keys_down.len()
    }
}

pub fn keyboard_interrupt_handler(_stack_frame: *mut crate::arch::ints::StackFrame) {
    unsafe {
        let mut keyboard_state = crate::game::WORLD
            .get_mut()
            .unwrap()
            .get_resource_mut::<KeyboardState>()
            .unwrap();
        keyboard_state
            .scancodes
            .push_back(crate::utils::asm::inb(0x60))
    };
    crate::arch::ints::pic::send_eoi(1);
}

// * i can probably do this inside keyboard_interrupt_handler with &mut World
pub fn keyboard_system(mut keyboard_state: ResMut<KeyboardState>) {
    keyboard_state.last_keys_down = keyboard_state.keys_down.clone();
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
