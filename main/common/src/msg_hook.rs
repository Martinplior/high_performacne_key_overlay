use std::time::Instant;

use windows::Win32::{
    Foundation::HWND,
    UI::{
        Input::KeyboardAndMouse::{VIRTUAL_KEY, VK_SHIFT},
        WindowsAndMessaging::{MSG, RI_KEY_BREAK, RI_KEY_E0, WM_INPUT},
    },
};

use crossbeam::channel::Sender as MpscSender;

use crate::{key::Key, key_message::KeyMessage, win_utils};

#[derive(Debug, Default)]
pub struct HookShared {
    pub egui_ctx: egui::Context,
}

pub fn create_register_raw_input_hook() -> impl FnOnce(&HWND) {
    |&hwnd| {
        use win_utils::raw_input_device;
        raw_input_device::register(
            raw_input_device::DeviceType::Keyboard,
            raw_input_device::OptionType::inputsink_with_no_legacy(hwnd),
        );
        raw_input_device::register(
            raw_input_device::DeviceType::Mouse,
            raw_input_device::OptionType::Remove,
        );
    }
}

pub fn create_msg_hook(
    msg_sender: MpscSender<KeyMessage>,
    hook_shared: HookShared,
) -> impl FnMut(&MSG) -> bool {
    move |msg| {
        if msg.message == WM_INPUT {
            handle_raw_input(msg, &msg_sender);
            let ctx = &hook_shared.egui_ctx;
            if !ctx.has_requested_repaint() {
                ctx.request_repaint();
            }
            return true;
        }
        false
    }
}

#[inline(always)]
fn handle_raw_input(msg: &MSG, msg_sender: &MpscSender<KeyMessage>) {
    let raw_input = win_utils::RawInput::from_msg(msg);
    let win_utils::RawInput::Keyboard(keyboard) = raw_input else {
        unreachable!("unexpeced raw input");
    };
    let keyboard = keyboard.data;
    let virtual_key = VIRTUAL_KEY(keyboard.VKey);
    let is_extend = if virtual_key == VK_SHIFT {
        keyboard.MakeCode == 0x0036
    } else {
        (keyboard.Flags & RI_KEY_E0 as u16) != 0
    };
    let key = Key::from_virtual_key(virtual_key, is_extend);
    if key == Key::Unknown {
        #[cfg(debug_assertions)]
        println!("vk = {:?}, is_ext = {:?}", virtual_key, is_extend);
        return;
    }
    let is_pressed = (keyboard.Flags & RI_KEY_BREAK as u16) == 0;
    let key_message = KeyMessage::new(key, is_pressed, Instant::now());
    #[cfg(debug_assertions)]
    println!("{:?}", key_message);
    msg_sender.send(key_message).unwrap();
}
