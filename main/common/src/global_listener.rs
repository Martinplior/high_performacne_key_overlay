use std::mem::ManuallyDrop;
use std::mem::MaybeUninit;
use std::thread::JoinHandle;

use windows::core::PCWSTR;
use windows::Win32::Foundation::HWND;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Foundation::LRESULT;
use windows::Win32::Foundation::WPARAM;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::CreateWindowExW;
use windows::Win32::UI::WindowsAndMessaging::DefWindowProcW;
use windows::Win32::UI::WindowsAndMessaging::DispatchMessageW;
use windows::Win32::UI::WindowsAndMessaging::GetMessageW;
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;
use windows::Win32::UI::WindowsAndMessaging::RegisterClassExW;
use windows::Win32::UI::WindowsAndMessaging::HWND_MESSAGE;
use windows::Win32::UI::WindowsAndMessaging::MSG;
use windows::Win32::UI::WindowsAndMessaging::WM_CLOSE;
use windows::Win32::UI::WindowsAndMessaging::WNDCLASSEXW;

use crossbeam::channel::Sender as MpscSender;

pub struct GlobalListener {
    msg_hwnd: HWND,
    thread: ManuallyDrop<JoinHandle<()>>,
}

impl GlobalListener {
    /// `msg_hook`: return true if you dont't want msg to be dispatched.
    /// `register_raw_input_hook`: register your raw input.
    pub fn new(
        msg_hook: impl FnMut(&MSG) -> bool + Send + 'static,
        register_raw_input_hook: impl FnOnce(&HWND) + Send + 'static,
    ) -> Self {
        Self::init_window_class();
        let (hwnd_sender, hwnd_receiver) = crossbeam::channel::bounded(1);
        let thread = std::thread::spawn(|| {
            Self::thread_main(msg_hook, register_raw_input_hook, hwnd_sender)
        });
        let msg_hwnd = hwnd_receiver.recv().unwrap();
        let msg_hwnd = HWND(msg_hwnd as _);
        Self {
            msg_hwnd,
            thread: ManuallyDrop::new(thread),
        }
    }

    const fn window_class_name() -> PCWSTR {
        windows::core::w!("global_listener_window_class")
    }

    fn init_window_class() {
        unsafe extern "system" fn wnd_proc(
            hwnd: HWND,
            msg: u32,
            wparam: WPARAM,
            lparam: LPARAM,
        ) -> LRESULT {
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }

        static INIT: std::sync::Once = std::sync::Once::new();
        INIT.call_once(|| {
            let window_class = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as _,
                lpfnWndProc: Some(wnd_proc),
                hInstance: unsafe { GetModuleHandleW(None) }.unwrap().into(),
                lpszClassName: Self::window_class_name(),
                ..Default::default()
            };
            if unsafe { RegisterClassExW(&window_class) } == 0 {
                panic!("{}", windows::core::Error::from_win32().message());
            }
        });
    }

    fn thread_main(
        mut msg_hook: impl FnMut(&MSG) -> bool,
        register_raw_input_hook: impl FnOnce(&HWND),
        hwnd_sender: MpscSender<usize>,
    ) {
        let hwnd = unsafe {
            CreateWindowExW(
                Default::default(),
                Self::window_class_name(),
                None,
                Default::default(),
                0,
                0,
                0,
                0,
                HWND_MESSAGE,
                None,
                None,
                None,
            )
        }
        .unwrap();

        hwnd_sender.send(hwnd.0 as _).unwrap();
        drop(hwnd_sender);

        register_raw_input_hook(&hwnd);

        loop {
            let mut msg = MaybeUninit::uninit();
            let r = unsafe { GetMessageW(msg.as_mut_ptr(), hwnd, 0, 0) }.0;
            match r {
                0 | -1 => break,
                _ => (),
            }
            let msg = unsafe { msg.assume_init() };
            if msg_hook(&msg) {
                continue;
            }
            unsafe { DispatchMessageW(&msg) };
        }
    }
}

impl Drop for GlobalListener {
    fn drop(&mut self) {
        unsafe { PostMessageW(self.msg_hwnd, WM_CLOSE, None, None) }.unwrap();
        unsafe { ManuallyDrop::take(&mut self.thread) }
            .join()
            .unwrap();
    }
}
