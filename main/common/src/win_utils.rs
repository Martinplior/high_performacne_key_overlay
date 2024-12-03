#![allow(dead_code)]

use std::mem::MaybeUninit;

use windows::Win32::UI::{
    Input::{
        GetRawInputData, HRAWINPUT, RAWHID, RAWINPUT, RAWINPUTHEADER, RAWKEYBOARD, RAWMOUSE,
        RID_DEVICE_INFO_TYPE, RID_INPUT, RIM_TYPEHID, RIM_TYPEKEYBOARD, RIM_TYPEMOUSE,
    },
    WindowsAndMessaging::MSG,
};

#[repr(C)]
#[derive(Clone)]
pub struct Keyboard {
    pub header: RAWINPUTHEADER,
    pub data: RAWKEYBOARD,
}

#[repr(C)]
#[derive(Clone)]
pub struct Mouse {
    pub header: RAWINPUTHEADER,
    pub data: RAWMOUSE,
}

#[repr(C)]
#[derive(Clone)]
pub struct HID {
    pub header: RAWINPUTHEADER,
    pub data: RAWHID,
}

#[derive(Clone)]
pub enum RawInput {
    Keyboard(Keyboard),
    Mouse(Mouse),
    HID(HID),
}

impl RawInput {
    pub fn from_msg(msg: &MSG) -> Self {
        let RAWINPUT { header, data } = {
            let mut raw_input = MaybeUninit::<RAWINPUT>::uninit();
            let mut size = std::mem::size_of::<RAWINPUT>() as _;
            let header_size = std::mem::size_of::<RAWINPUTHEADER>() as _;
            let r = unsafe {
                GetRawInputData(
                    HRAWINPUT(msg.lParam.0 as _),
                    RID_INPUT,
                    Some(raw_input.as_mut_ptr() as _),
                    &mut size,
                    header_size,
                )
            };
            if r == u32::MAX {
                panic!("{}", windows::core::Error::from_win32().message());
            }
            unsafe { raw_input.assume_init() }
        };
        match RID_DEVICE_INFO_TYPE(header.dwType) {
            RIM_TYPEKEYBOARD => Self::Keyboard(Keyboard {
                header,
                data: unsafe { data.keyboard },
            }),
            RIM_TYPEMOUSE => Self::Mouse(Mouse {
                header,
                data: unsafe { data.mouse },
            }),
            RIM_TYPEHID => Self::HID(HID {
                header,
                data: unsafe { data.hid },
            }),
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }
}

pub mod raw_input_device {
    use windows::Win32::{
        Devices::HumanInterfaceDevice::{
            HID_USAGE_GENERIC_KEYBOARD, HID_USAGE_GENERIC_MOUSE, HID_USAGE_PAGE_GENERIC,
        },
        Foundation::HWND,
        UI::Input::{
            RegisterRawInputDevices, RAWINPUTDEVICE, RAWINPUTDEVICE_FLAGS, RIDEV_INPUTSINK,
            RIDEV_NOLEGACY, RIDEV_REMOVE,
        },
    };

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum DeviceType {
        Keyboard,
        Mouse,
    }

    type Flags = RAWINPUTDEVICE_FLAGS;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum OptionType {
        Remove,
        Flags(HWND, Flags),
    }

    impl OptionType {
        pub fn inputsink(hwnd: HWND) -> Self {
            Self::Flags(hwnd, RIDEV_INPUTSINK)
        }

        pub fn inputsink_with_no_legacy(hwnd: HWND) -> Self {
            Self::Flags(hwnd, RIDEV_INPUTSINK | RIDEV_NOLEGACY)
        }
    }

    pub fn register(device_type: DeviceType, option_type: OptionType) {
        let device = match device_type {
            DeviceType::Keyboard => HID_USAGE_GENERIC_KEYBOARD,
            DeviceType::Mouse => HID_USAGE_GENERIC_MOUSE,
        };
        let (hwnd, flags) = match option_type {
            OptionType::Remove => (HWND(std::ptr::null_mut()), RIDEV_REMOVE),
            OptionType::Flags(hwnd, flags) => (hwnd, flags),
        };
        let raw_input_device = RAWINPUTDEVICE {
            usUsagePage: HID_USAGE_PAGE_GENERIC,
            usUsage: device,
            dwFlags: flags,
            hwndTarget: hwnd,
        };
        unsafe {
            RegisterRawInputDevices(
                &[raw_input_device],
                std::mem::size_of::<RAWINPUTDEVICE>() as _,
            )
        }
        .unwrap();
    }
}
