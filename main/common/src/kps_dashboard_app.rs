use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use eframe::{
    egui_wgpu::WgpuConfiguration,
    wgpu::{Backends, PowerPreference, PresentMode},
};
use egui::{Color32, TextureHandle, ViewportBuilder};
use serde::{Deserialize, Serialize};

use crate::{
    global_listener::GlobalListener, key::Key, key_message::KeyMessage, message_dialog, msg_hook,
};

use crossbeam::channel::Receiver as MpscReceiver;

pub struct MainApp {
    kps_setting: KpsSetting,
}

impl MainApp {
    const EDGE: f32 = 600.0;

    pub fn new() -> Self {
        Self {
            kps_setting: KpsSetting::load_from_local_setting(),
        }
    }

    pub fn run(self) {
        let edge = Self::EDGE;
        let icon_data = {
            let img = image::load_from_memory(include_bytes!("../../icons/kps_icon.png")).unwrap();
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
                .with_inner_size([edge, edge])
                .with_resizable(false)
                .with_maximize_button(false)
                .with_minimize_button(false)
                .with_icon(icon_data),
            renderer: eframe::Renderer::Wgpu,
            wgpu_options: WgpuConfiguration {
                supported_backends: Backends::VULKAN,
                present_mode: PresentMode::AutoVsync,
                power_preference: PowerPreference::HighPerformance,
                ..Default::default()
            },
            ..Default::default()
        };
        eframe::run_native(
            "HP KPS Dashboard",
            native_options,
            Box::new(|cc| Ok(Box::new(App::new(cc, self.kps_setting)))),
        )
        .unwrap();
    }
}

struct App {
    kps: Kps,
    background_color: Color32,
    keys_receiver: MpscReceiver<KeyMessage>,
    keys_message_buf: Vec<KeyMessage>,
    key_repeat_flags: [bool; Self::KEY_REPEAT_FLAGS_CAP],
    _global_listener: GlobalListener,
}

impl App {
    const KEY_REPEAT_FLAGS_CAP: usize = Key::LAST_KEY as usize;

    pub fn new(cc: &eframe::CreationContext<'_>, kps_setting: KpsSetting) -> Self {
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
        let [r, g, b] = kps_setting.background_color;
        Self {
            kps: Kps::new(&cc.egui_ctx, kps_setting),
            background_color: Color32::from_rgb(r, g, b),
            keys_receiver,
            keys_message_buf: Vec::with_capacity(64),
            key_repeat_flags: [false; Self::KEY_REPEAT_FLAGS_CAP],
            _global_listener: global_listener,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let instant_now = std::time::Instant::now();
        self.keys_message_buf.extend(self.keys_receiver.try_iter());
        self.keys_message_buf
            .drain(..)
            .filter(|key_message| {
                let index = key_message.key as usize;
                let flag = self.key_repeat_flags.get_mut(index).unwrap();
                let old_flag = *flag;
                let is_pressed = key_message.is_pressed;
                *flag = is_pressed;
                !old_flag && is_pressed
            })
            .for_each(|key_message| self.kps.update(&key_message));
        self.kps.remove_outer_key(instant_now);
        let stable_dt = ctx.input(|i| i.stable_dt.min(i.predicted_dt));
        self.kps.update_pointer_value(stable_dt);

        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(self.background_color))
            .show(ctx, |ui| self.kps.show(ui));

        self.kps.need_repaint().then(|| ctx.request_repaint());
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct KpsSetting {
    /// unit: ms
    interval_ms: f32,
    max_count: u32,
    background_color: [u8; 3],
}

impl Default for KpsSetting {
    fn default() -> Self {
        Self {
            interval_ms: 250.0,
            max_count: 24,
            background_color: [0, 255, 0],
        }
    }
}

impl KpsSetting {
    fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self, &'static str> {
        let file = std::fs::File::options()
            .read(true)
            .open(path)
            .map_err(|_| "无法读取文件")?;
        let reader = std::io::BufReader::new(&file);
        let setting = serde_json::de::from_reader(reader).map_err(|_| "格式错误")?;
        Ok(setting)
    }

    fn to_file(self, path: impl AsRef<std::path::Path>) -> Result<(), String> {
        let file = std::fs::File::create(path).map_err(|_| "无法写入文件")?;
        let writer = std::io::BufWriter::new(&file);
        serde_json::ser::to_writer_pretty(writer, &self)
            .map_err(|err| format!("serde_json::ser::to_writer_pretty错误：{}", err))?;
        Ok(())
    }

    fn load_from_local_setting() -> Self {
        let path = crate::get_current_dir().join("kps_setting.json");
        Self::from_file(&path).unwrap_or_else(|_| {
            let setting = Self::default();
            let _ = setting
                .clone()
                .to_file(path)
                .map(|_| {
                    message_dialog::warning("读取配置文件失败，已生成默认配置").show();
                })
                .map_err(|_| {
                    message_dialog::warning("读取配置文件失败，且无法生成配置文件，使用默认配置")
                        .show();
                });
            setting
        })
    }
}

struct Kps {
    key_instant_queue: VecDeque<Instant>,
    interval: Duration,
    max_count: u32,
    pointer_value: f32,
    kps_frame_handle: TextureHandle,
    kps_pointer_handle: TextureHandle,
}

impl Kps {
    fn new(egui_ctx: &egui::Context, setting: KpsSetting) -> Self {
        let (kps_frame_img, kps_pointer_img) = {
            let kps_frame_img =
                image::load_from_memory(include_bytes!("../../textures/kps_frame.png")).unwrap();
            let kps_pointer_img =
                image::load_from_memory(include_bytes!("../../textures/kps_pointer.png")).unwrap();
            let kps_frame_img = egui::ColorImage::from_rgba_unmultiplied(
                [
                    kps_frame_img.width() as usize,
                    kps_frame_img.height() as usize,
                ],
                kps_frame_img.as_bytes(),
            );
            let kps_pointer_img = egui::ColorImage::from_rgba_unmultiplied(
                [
                    kps_pointer_img.width() as usize,
                    kps_pointer_img.height() as usize,
                ],
                kps_pointer_img.as_bytes(),
            );
            (kps_frame_img, kps_pointer_img)
        };
        let kps_frame_handle =
            egui_ctx.load_texture("kps_frame", kps_frame_img, egui::TextureOptions::LINEAR);
        let kps_pointer_handle =
            egui_ctx.load_texture("kps_pointer", kps_pointer_img, egui::TextureOptions::LINEAR);

        Self {
            key_instant_queue: VecDeque::with_capacity(64),
            interval: Duration::from_secs_f32(setting.interval_ms.clamp(0.0, 5000.0) / 1_000.0),
            max_count: setting.max_count.max(1),
            pointer_value: 0.0,
            kps_frame_handle,
            kps_pointer_handle,
        }
    }

    fn show(&mut self, ui: &mut egui::Ui) {
        let edge = MainApp::EDGE;
        let painter = ui.painter();

        let uv = egui::Rect::from_min_max([0.0, 0.0].into(), [1.0, 1.0].into());

        // frame
        painter.image(
            self.kps_frame_handle.id(),
            ui.clip_rect(),
            uv,
            Color32::WHITE,
        );

        let font_id = egui::FontId::monospace(100.0);
        let text_color = Color32::from_rgb(0xfb, 0xfb, 0xfb);

        // KPS text
        painter.text(
            [edge / 2.0, 200.0].into(),
            egui::Align2::CENTER_CENTER,
            "KPS",
            font_id.clone(),
            text_color,
        );

        // counter text
        painter.text(
            [edge / 2.0, 400.0].into(),
            egui::Align2::CENTER_CENTER,
            format!("{}", self.count()),
            font_id,
            text_color,
        );

        // pointer
        let start_angle = -120.0;
        let end_angle = 120.0;
        let range = end_angle - start_angle;
        let ratio = (self.pointer_value / self.max_count as f32).clamp(0.0, 1.0);

        let pointer_angle = (start_angle + range * ratio).clamp(start_angle, end_angle);
        let pointer_angle_rad = pointer_angle / 180.0 * std::f32::consts::PI;

        let mut pointer_mesh = epaint::Mesh::with_texture(self.kps_pointer_handle.id());
        pointer_mesh.add_rect_with_uv(ui.clip_rect(), uv, Color32::WHITE);
        pointer_mesh.rotate(
            egui::emath::Rot2::from_angle(pointer_angle_rad),
            ui.clip_rect().center(),
        );

        painter.add(egui::Shape::mesh(pointer_mesh));
    }

    fn update(&mut self, key_message: &KeyMessage) {
        self.key_instant_queue.push_back(key_message.instant);
    }

    fn remove_outer_key(&mut self, instant_now: Instant) {
        let dead_line = instant_now - Duration::from_secs(1);
        let count = self
            .key_instant_queue
            .iter()
            .take_while(|&instant| *instant < dead_line)
            .count();
        self.key_instant_queue.drain(..count);
    }

    fn count(&self) -> u32 {
        self.key_instant_queue.len() as u32
    }

    const MIN_ABS_DIFF: f32 = 0.01;

    fn update_pointer_value(&mut self, stable_dt: f32) {
        let ratio = stable_dt / self.interval.as_secs_f32();
        let mut diff = self.count() as f32 - self.pointer_value;
        if diff.abs() < Self::MIN_ABS_DIFF {
            diff = diff.signum() * Self::MIN_ABS_DIFF;
        }
        self.pointer_value += diff * ratio;
        if diff.is_sign_positive() {
            self.pointer_value = self.pointer_value.min(self.count() as f32);
        } else {
            self.pointer_value = self.pointer_value.max(self.count() as f32);
        }
    }

    fn need_repaint(&self) -> bool {
        self.pointer_value > f32::EPSILON
    }
}
