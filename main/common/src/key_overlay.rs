use std::time::Instant;

use crate::{
    key::Key,
    key_bar::KeyBar,
    key_drawer::KeyDrawer,
    key_message::KeyMessage,
    key_property::KeyProperty,
    setting::{self, Setting},
};
use egui::{Color32, FontData, FontDefinitions, FontFamily, Rounding, TextureHandle};

use crossbeam::channel::Receiver as MpscReceiver;

#[derive(Debug)]
struct KeyMap {
    map: Box<[Option<Box<[usize]>>; Self::CAP]>,
}

impl KeyMap {
    const CAP: usize = Key::LAST_KEY as usize;

    fn new(key_properties: &[KeyProperty]) -> Self {
        let init = || -> Option<_> {
            let mut init_map: Box<[_; Self::CAP]> = Box::new(std::array::from_fn(|_| Some(vec![])));
            let iter = key_properties
                .iter()
                .filter(|key_property| key_property.key_bind != Key::Unknown)
                .enumerate();
            for (index, key_property) in iter {
                let indexes = init_map.get_mut(key_property.key_bind as usize)?.as_mut()?;
                indexes.push(index);
            }
            let map = Box::new(std::array::from_fn(|index| {
                let init_vec = init_map.get_mut(index).unwrap().take().unwrap();
                if init_vec.is_empty() {
                    None
                } else {
                    Some(init_vec.into_boxed_slice())
                }
            }));
            Some(map)
        };

        let map = init().unwrap();

        Self { map }
    }

    fn get(&self, key: Key) -> Option<&[usize]> {
        self.map.get(key as usize).unwrap().as_deref()
    }
}

struct KeyHandler {
    key_properties: Box<[KeyProperty]>,
    key_drawers: Box<[KeyDrawer]>,
    key_map: KeyMap,
    font_family: egui::FontFamily,
    background_color: setting::BackgroundColor,
    /// `[up, down, left, right]`
    fade_mask_handles: [TextureHandle; 4],
}

impl KeyHandler {
    fn new(egui_ctx: &egui::Context, setting: Setting) -> Self {
        let Setting {
            window_setting,
            background_color,
            key_properties,
            ..
        } = setting;
        let key_properties = key_properties.into_boxed_slice();
        let key_map = KeyMap::new(&key_properties);
        let key_drawers = key_properties
            .iter()
            .map(|key_property| {
                KeyDrawer::new(&window_setting, key_property.bar_speed, key_property)
            })
            .collect();
        let font_family = egui::FontFamily::Name(KeyOverlay::FONT_FAMILY_NAME.into());
        let fade_mask_handles = Self::init_fade_mask(egui_ctx, background_color.into());
        Self {
            key_properties,
            key_drawers,
            key_map,
            font_family,
            background_color,
            fade_mask_handles,
        }
    }

    fn reload(&mut self, egui_ctx: &egui::Context, setting: &Setting) {
        let Setting {
            window_setting,
            background_color,
            key_properties,
            ..
        } = setting;
        let new_key_properties = key_properties.clone().into_boxed_slice();
        let new_key_map = KeyMap::new(&new_key_properties);
        let new_key_drawers = new_key_properties
            .iter()
            .map(|key_property| {
                KeyDrawer::new(&window_setting, key_property.bar_speed, key_property)
            })
            .collect();

        if self.background_color != *background_color {
            let new_fade_mask_handles =
                Self::init_fade_mask(egui_ctx, background_color.clone().into());
            self.fade_mask_handles = new_fade_mask_handles;
            self.background_color = *background_color;
        }

        self.key_properties = new_key_properties;
        self.key_map = new_key_map;
        self.key_drawers = new_key_drawers;
    }

    /// returns `[up_mask, down_mask, left_mask, right_mask]`
    fn init_fade_mask(
        egui_ctx: &egui::Context,
        background_color: setting::BackgroundColor,
    ) -> [TextureHandle; 4] {
        let masks = {
            let color: Color32 = background_color.into();
            let [r, g, b, ..] = color.to_array();
            let rgba_iter = (0..=255).map(|a| [r, g, b, a]);
            let rgba: Box<_> = rgba_iter.clone().flatten().collect();
            let rgba_rev: Box<_> = rgba_iter.rev().flatten().collect();
            let up_mask = egui::ColorImage::from_rgba_unmultiplied([1, 256], &*rgba_rev);
            let down_mask = egui::ColorImage::from_rgba_unmultiplied([1, 256], &*rgba);
            let left_mask = egui::ColorImage::from_rgba_unmultiplied([256, 1], &*rgba_rev);
            let right_mask = egui::ColorImage::from_rgba_unmultiplied([256, 1], &*rgba);
            [up_mask, down_mask, left_mask, right_mask]
        };
        let names = ["up_mask", "down_mask", "left_mask", "right_mask"];
        let mut iter = masks
            .into_iter()
            .zip(names.into_iter())
            .map(|(mask, name)| egui_ctx.load_texture(name, mask, egui::TextureOptions::LINEAR));
        std::array::from_fn(|_| iter.next().unwrap())
    }

    fn update(&mut self, key_message: KeyMessage) {
        debug_assert!(key_message.key != Key::Unknown);

        let mut inner_update = |indexes: &[usize]| -> Option<()> {
            let first_key_drawer = self.key_drawers.get_mut(*indexes.first()?)?;

            let now_pressed = key_message.is_pressed;
            let prev_pressed = first_key_drawer.begin_hold_instant.is_some();

            match (prev_pressed, now_pressed) {
                (false, true) => {
                    for index in indexes.iter() {
                        let key_property = self.key_properties.get_mut(*index)?;
                        let key_drawer = self.key_drawers.get_mut(*index)?;
                        if key_property.key_counter.0 {
                            key_drawer.increase_count();
                        }
                        key_drawer.begin_hold_instant = Some(key_message.instant);
                    }
                }
                (true, false) => {
                    for index in indexes.iter() {
                        let key_drawer = self.key_drawers.get_mut(*index)?;
                        let bar =
                            KeyBar::new(key_drawer.begin_hold_instant.take()?, key_message.instant);
                        key_drawer.add_bar(bar);
                    }
                }
                _ => (),
            }
            Some(())
        };

        self.key_map
            .get(key_message.key)
            .map(|indexes| unsafe { inner_update(indexes).unwrap_unchecked() });
    }

    fn remove_outer_bar(&mut self, instant_now: Instant) {
        self.key_drawers.iter_mut().for_each(|key_drawer| {
            key_drawer.remove_outer_bar(instant_now);
        });
    }

    fn draw_on(&self, painter: &egui::Painter, instant_now: Instant) {
        let fade_mask_id = std::array::from_fn(|index| self.fade_mask_handles[index].id());
        self.key_properties
            .iter()
            .zip(self.key_drawers.iter())
            .for_each(|(key_property, key_drawer)| {
                key_drawer.draw_on(
                    painter,
                    key_property,
                    instant_now,
                    &self.font_family,
                    fade_mask_id,
                );
            });
    }
}

pub struct KeyOverlay {
    instant_now: Instant,
    key_messages_buffer: Vec<KeyMessage>,
    keys_receiver: MpscReceiver<KeyMessage>,
    egui_ctx: egui::Context,
    background_color: Color32,
    key_handler: KeyHandler,
}

impl KeyOverlay {
    pub const FONT_FAMILY_NAME: &str = "key_overlay_font";

    pub fn new(
        egui_ctx: &egui::Context,
        setting: Setting,
        keys_receiver: MpscReceiver<KeyMessage>,
    ) -> Self {
        let Setting {
            font_name,
            background_color,
            ..
        } = &setting;

        let background_color = background_color.clone().into();

        Self::init_fonts(egui_ctx, &font_name);

        let instant_now = Instant::now();
        let key_handler = KeyHandler::new(egui_ctx, setting);
        Self {
            instant_now,
            key_messages_buffer: Vec::with_capacity(64),
            keys_receiver,
            egui_ctx: egui_ctx.clone(),
            background_color,
            key_handler,
        }
    }

    pub fn load_setting(&mut self, setting: Setting, reload_font: bool) {
        let Setting {
            font_name,
            background_color,
            ..
        } = &setting;

        let new_background_color = background_color.clone().into();

        reload_font.then(|| Self::init_fonts(&self.egui_ctx, &font_name));

        self.background_color = new_background_color;
        self.key_handler.reload(&self.egui_ctx, &setting);
    }

    fn init_fonts(egui_ctx: &egui::Context, font_family: &str) {
        let sys_fonts = font_kit::source::SystemSource::new();
        let mut font_definitions = FontDefinitions::default();
        let font_list = [font_family, Setting::DEFAULT_FONT_NAME, "Segoe UI emoji"];
        let font_list_iter = font_list.iter().filter_map(|font_family| {
            let Ok(family_handle) = sys_fonts.select_family_by_name(font_family) else {
                return None;
            };
            let first_font_handle = family_handle.fonts().first()?;
            let is_ttc = match first_font_handle {
                font_kit::handle::Handle::Path { path, .. } => {
                    matches!(
                        font_kit::font::Font::analyze_path(path).ok()?,
                        font_kit::file_type::FileType::Collection(_)
                    )
                }
                _ => return None,
            };
            let font_data = first_font_handle.load().ok()?.copy_font_data()?;
            let font_data = if is_ttc {
                owned_ttf_parser::OwnedFace::from_vec((*font_data).clone(), 0)
                    .ok()?
                    .into_vec()
            } else {
                (*font_data).clone()
            };
            Some((font_data, font_family.to_string()))
        });

        let font_family_name = egui::FontFamily::Name(Self::FONT_FAMILY_NAME.into());
        let default_proportional = font_definitions
            .families
            .get(&FontFamily::Proportional)
            .unwrap()
            .clone();

        let mut custom_font_names: Vec<_> = font_list_iter
            .map(|(font_data, font_name)| {
                font_definitions
                    .font_data
                    .insert(font_name.clone(), FontData::from_owned(font_data));
                font_name
            })
            .collect();
        custom_font_names.extend(default_proportional.clone().into_iter());
        font_definitions
            .families
            .insert(font_family_name.clone(), custom_font_names);

        let mut font_names: Vec<_> = font_list[1..].iter().map(|x| x.to_string()).collect();
        font_names.extend(default_proportional.into_iter());
        font_definitions
            .families
            .insert(egui::FontFamily::Proportional, font_names);

        egui_ctx.set_fonts(font_definitions);
    }

    pub fn update(&mut self, instant_now: Instant) {
        self.instant_now = instant_now;
        self.key_messages_buffer
            .extend(self.keys_receiver.try_iter());
        self.key_messages_buffer.drain(..).for_each(|key_message| {
            self.key_handler.update(key_message);
        });
        self.key_handler.remove_outer_bar(instant_now);
    }

    pub fn keys_receiver(&mut self) -> &mut MpscReceiver<KeyMessage> {
        &mut self.keys_receiver
    }

    pub fn show(&self, ui: &mut egui::Ui) {
        let painter = ui.painter();
        painter.rect_filled(painter.clip_rect(), Rounding::ZERO, self.background_color);
        self.key_handler.draw_on(painter, self.instant_now);
    }

    pub fn need_repaint(&self) -> bool {
        self.key_handler
            .key_drawers
            .iter()
            .any(|key_drawer| key_drawer.need_repaint())
    }
}
