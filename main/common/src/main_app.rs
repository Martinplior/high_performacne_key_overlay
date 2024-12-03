use eframe::{
    egui_wgpu::WgpuConfiguration,
    wgpu::{Backends, PowerPreference, PresentMode},
};
use egui::ViewportBuilder;

use crate::{
    global_listener::GlobalListener,
    key_overlay::KeyOverlay,
    msg_hook,
    setting::{Setting, WindowSetting},
};

pub struct MainApp {
    setting: Option<Setting>,
}

impl MainApp {
    pub fn new() -> Self {
        Self {
            setting: Some(Setting::load_from_local_setting()),
        }
    }

    pub fn run(mut self) {
        let setting = self.setting.take().unwrap();
        let WindowSetting {
            width,
            height,
            enable_vsync,
        } = setting.window_setting;

        let icon_data = {
            let img = image::load_from_memory(include_bytes!("../../icons/main_icon.png")).unwrap();
            let width = img.width();
            let height = img.height();
            let data = img.into_bytes();
            egui::IconData {
                rgba: data,
                width,
                height,
            }
        };
        let native_options = eframe::NativeOptions {
            viewport: ViewportBuilder::default()
                .with_inner_size([width, height])
                .with_resizable(false)
                .with_maximize_button(false)
                .with_minimize_button(false)
                .with_icon(icon_data),
            renderer: eframe::Renderer::Wgpu,
            wgpu_options: WgpuConfiguration {
                supported_backends: Backends::VULKAN,
                present_mode: if enable_vsync {
                    PresentMode::AutoVsync
                } else {
                    PresentMode::AutoNoVsync
                },
                power_preference: PowerPreference::HighPerformance,
                ..Default::default()
            },
            ..Default::default()
        };
        eframe::run_native(
            "HP KeyOverlay",
            native_options,
            Box::new(|cc| Ok(Box::new(App::new(cc, setting)))),
        )
        .unwrap();
    }
}

struct App {
    key_overlay: KeyOverlay,
    _global_listener: GlobalListener,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, setting: Setting) -> Self {
        cc.egui_ctx.request_repaint();
        let cap = crate::CHANNEL_CAP;
        let (keys_sender, keys_receiver) = crossbeam::channel::bounded(cap);
        let hook_shared = msg_hook::HookShared {
            egui_ctx: cc.egui_ctx.clone(),
        };
        let global_listener = GlobalListener::new(
            msg_hook::create_msg_hook(keys_sender, hook_shared),
            msg_hook::create_register_raw_input_hook(),
        );
        let key_overlay = KeyOverlay::new(&cc.egui_ctx, setting, keys_receiver);
        Self {
            key_overlay,
            _global_listener: global_listener,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let instant_now = std::time::Instant::now();
        self.key_overlay.update(instant_now);
        egui::CentralPanel::default().show(ctx, |ui| self.key_overlay.show(ui));
        self.key_overlay
            .need_repaint()
            .then(|| ctx.request_repaint());
    }
}
