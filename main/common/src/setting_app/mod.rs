use std::sync::Arc;

use eframe::{
    egui_wgpu::WgpuConfiguration,
    wgpu::{Backends, PowerPreference, PresentMode},
};
use egui::ViewportBuilder;

use crate::{global_listener::GlobalListener, key_overlay::KeyOverlay, msg_hook, setting::Setting};

mod menu;
mod setting_area;

pub struct SettingApp {
    setting: Option<Setting>,
}

impl SettingApp {
    pub fn new() -> Self {
        Self {
            setting: Some(Setting::load_from_local_setting()),
        }
    }

    pub fn run(mut self) {
        let setting = self.setting.take().unwrap();
        let enable_vsync = setting.window_setting.enable_vsync;
        let min_edge = App::WINDOW_MIN_EDGE;
        let edge = min_edge + 200.0;
        let icon_data = {
            let img =
                image::load_from_memory(include_bytes!("../../../icons/setting_icon.png")).unwrap();
            let width = img.width();
            let height = img.height();
            let data = img.into_bytes();
            Arc::new(egui::IconData {
                rgba: data,
                width,
                height,
            })
        };
        let native_options = eframe::NativeOptions {
            viewport: ViewportBuilder::default()
                .with_min_inner_size([min_edge, min_edge])
                .with_inner_size([edge, edge])
                .with_icon(icon_data.clone()),
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
            "设置",
            native_options,
            Box::new(|cc| Ok(Box::new(App::new(cc, setting, icon_data)))),
        )
        .unwrap();
    }
}

struct AppSharedData {
    load_path: std::path::PathBuf,
    /// setting that is loaded from file
    loaded_setting: Setting,
    /// setting that is using now
    current_setting: Setting,
    /// setting that to be reloaded
    pending_setting: Option<Setting>,
    modified: bool,

    key_overlay: KeyOverlay,
}

struct App {
    shared_data: AppSharedData,
    _global_listener: GlobalListener,
    icon_data: Arc<egui::IconData>,
    menu: menu::Menu,
    setting_area: setting_area::SettingArea,
}

impl App {
    fn new(
        cc: &eframe::CreationContext<'_>,
        setting: Setting,
        icon_data: Arc<egui::IconData>,
    ) -> Self {
        cc.egui_ctx.set_theme(egui::ThemePreference::Dark);
        let cap = crate::CHANNEL_CAP;
        let (keys_sender, keys_receiver) = crossbeam::channel::bounded(cap);
        let hook_shared = msg_hook::HookShared {
            egui_ctx: cc.egui_ctx.clone(),
        };
        let global_listener = GlobalListener::new(
            msg_hook::create_msg_hook(keys_sender, hook_shared),
            msg_hook::create_register_raw_input_hook(),
        );
        let key_overlay = KeyOverlay::new(&cc.egui_ctx, setting.clone(), keys_receiver);
        let menu = menu::Menu::new();
        let setting_area = setting_area::SettingArea::new(&setting);
        let shared_data = AppSharedData {
            load_path: crate::key_overlay_setting_path(),
            loaded_setting: setting.clone(),
            current_setting: setting,
            pending_setting: None,
            modified: false,
            key_overlay,
        };
        Self {
            shared_data,
            _global_listener: global_listener,
            icon_data,
            menu,
            setting_area,
        }
    }
}

impl App {
    const WINDOW_MIN_EDGE: f32 = 600.0;
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let instant_now = std::time::Instant::now();
        self.try_load_pending_setting();

        self.show(ctx);
        self.show_keyoverlay(ctx);

        self.menu.update(ctx, &mut self.shared_data);
        self.setting_area.update(&mut self.shared_data);
        self.shared_data.key_overlay.update(instant_now);

        self.shared_data
            .key_overlay
            .need_repaint()
            .then(|| ctx.request_repaint());
    }
}

impl App {
    fn try_load_pending_setting(&mut self) {
        self.shared_data
            .pending_setting
            .take()
            .map(|pending_setting| {
                let reload_font =
                    pending_setting.font_name != self.shared_data.current_setting.font_name;
                self.shared_data
                    .key_overlay
                    .load_setting(pending_setting.clone(), reload_font);
                self.setting_area.reload(&pending_setting);
                self.shared_data.current_setting = pending_setting;
                self.shared_data.modified =
                    self.shared_data.current_setting != self.shared_data.loaded_setting;
            });
    }

    fn show(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu")
            .resizable(false)
            .show(ctx, |ui| self.menu.show(ui));
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .show(ui, |ui| self.setting_area.show(ui));
        });
    }

    fn show_keyoverlay(&mut self, ctx: &egui::Context) {
        let new_viewport_id = egui::ViewportId::from_hash_of("keyoverlay");
        let window_setting = &self.shared_data.current_setting.window_setting;
        let viewport_builder = egui::ViewportBuilder::default()
            .with_minimize_button(false)
            .with_maximize_button(false)
            .with_close_button(false)
            .with_icon(self.icon_data.clone())
            .with_title("预览")
            .with_resizable(false)
            .with_inner_size(egui::vec2(window_setting.width, window_setting.height));
        ctx.show_viewport_immediate(new_viewport_id, viewport_builder, |ctx, _vc| {
            egui::CentralPanel::default().show(ctx, |ui| self.shared_data.key_overlay.show(ui));
        });
    }
}
