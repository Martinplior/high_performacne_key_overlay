use std::ptr::NonNull;

use egui::Widget;

use crate::{
    key::Key,
    key_message::KeyMessage,
    key_overlay::KeyOverlay,
    key_property::{KeyDirection, KeyProperty},
    message_dialog,
    setting::{BackgroundColor, Setting, WindowSetting},
};

use super::AppSharedData;

use crossbeam::channel::Receiver as MpscReceiver;

macro_rules! grid_new_row {
    ($ui:ident, $b: block) => {{
        $b;
        $ui.end_row();
    }};
}

pub struct SettingArea {
    request_reload_setting: bool,
    window_setting_row: WindowSettingRow,
    key_property_setting_row: KeyPropertySettingRow,
}

impl SettingArea {
    pub fn new(setting: &Setting) -> Self {
        Self {
            request_reload_setting: false,
            window_setting_row: WindowSettingRow::new(setting),
            key_property_setting_row: KeyPropertySettingRow::new(setting),
        }
    }

    pub fn reload(&mut self, setting: &Setting) {
        self.window_setting_row.reload(setting);
        self.key_property_setting_row.reload(setting);
    }

    pub fn update(&mut self, app_shared_data: &mut AppSharedData) {
        self.window_setting_row
            .update(&mut self.request_reload_setting);
        self.key_property_setting_row.update(
            &mut self.request_reload_setting,
            app_shared_data.key_overlay.keys_receiver(),
        );
        std::mem::take(&mut self.request_reload_setting).then(|| {
            let WindowSettingRow {
                window_setting,
                background_color,
                current_font_family: current_font_name,
                ..
            } = &self.window_setting_row;
            let KeyPropertySettingRow { key_properties, .. } = &self.key_property_setting_row;
            let setting = Setting {
                window_setting: window_setting.clone(),
                font_name: current_font_name.clone(),
                background_color: *background_color,
                key_properties: key_properties.clone(),
            };
            app_shared_data.pending_setting = Some(setting);
        });
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            self.window_setting_row.show(ui);
            ui.separator();
            self.key_property_setting_row.show(ui);
        });
    }
}

struct WindowSettingRow {
    window_setting: WindowSetting,
    background_color: BackgroundColor,
    current_font_family: Box<str>,
    font_families: Box<[Box<str>]>,
    request_reload: bool,
}

impl WindowSettingRow {
    const MAX_EDGE: f32 = 8192.0;

    fn new(setting: &Setting) -> Self {
        let font_families = {
            let system_source = font_kit::source::SystemSource::new();
            if let Ok(families) = system_source.all_families() {
                let mut font_families: Box<[_]> =
                    families.into_iter().map(|x| x.into_boxed_str()).collect();
                font_families.sort();
                font_families
            } else {
                message_dialog::warning("加载系统字体失败！").show();
                vec![].into_boxed_slice()
            }
        };
        Self {
            window_setting: setting.window_setting.clone(),
            background_color: setting.background_color,
            current_font_family: setting.font_name.clone(),
            font_families,
            request_reload: false,
        }
    }

    fn reload(&mut self, setting: &Setting) {
        self.window_setting = setting.window_setting.clone();
        self.background_color = setting.background_color;
        self.current_font_family = setting.font_name.clone();
    }

    fn update(&mut self, request_reload: &mut bool) {
        *request_reload |= std::mem::take(&mut self.request_reload);
    }

    fn show(&mut self, ui: &mut egui::Ui) {
        fn layout<R>(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> R {
            let frame = egui::Frame::default()
                .inner_margin(egui::Margin::same(5.0))
                .stroke(ui.visuals().noninteractive().bg_stroke);
            frame.show(ui, |ui| ui.horizontal(add_contents).inner).inner
        }

        let changed = layout(ui, |ui| {
            let mut changed = false;
            let grid_left = egui::Grid::new(ui.next_auto_id())
                .min_col_width(0.0)
                .striped(true);
            grid_left.show(ui, |ui| {
                grid_new_row!(ui, {
                    egui::Label::new("窗口宽度:")
                        .selectable(false)
                        .ui(ui)
                        .on_hover_text("KeyOverlay窗口的宽度");
                    changed |=
                        egui::Slider::new(&mut self.window_setting.width, 1.0..=Self::MAX_EDGE)
                            .integer()
                            .drag_value_speed(1.0)
                            .logarithmic(true)
                            .ui(ui)
                            .changed();
                });
                grid_new_row!(ui, {
                    egui::Label::new("窗口高度:")
                        .selectable(false)
                        .ui(ui)
                        .on_hover_text("KeyOverlay窗口的高度");
                    changed |=
                        egui::Slider::new(&mut self.window_setting.height, 1.0..=Self::MAX_EDGE)
                            .integer()
                            .drag_value_speed(1.0)
                            .logarithmic(true)
                            .ui(ui)
                            .changed();
                });
                grid_new_row!(ui, {
                    egui::Label::new("垂直同步:")
                        .selectable(false)
                        .ui(ui)
                        .on_hover_text(concat!(
                            "关闭垂直同步基本没有益处...\n",
                            "也许你能用它来测试性能？\n",
                            "注意:该选项在修改/保存后不会立即生效"
                        ));
                    changed |= egui::Checkbox::without_text(&mut self.window_setting.enable_vsync)
                        .ui(ui)
                        .changed();
                });
            });

            ui.separator();

            let grid_right = egui::Grid::new(ui.next_auto_id())
                .min_col_width(0.0)
                .striped(true);
            grid_right.show(ui, |ui| {
                grid_new_row!(ui, {
                    egui::Label::new("背景颜色:")
                        .selectable(false)
                        .ui(ui)
                        .on_hover_text("只能支持8种颜色，不要问为什么...");
                    ui.horizontal(|ui| {
                        changed |= ui.toggle_value(&mut self.background_color.r, "R").changed();
                        changed |= ui.toggle_value(&mut self.background_color.g, "G").changed();
                        changed |= ui.toggle_value(&mut self.background_color.b, "B").changed();
                    });
                });
                grid_new_row!(ui, {
                    egui::Label::new("字体:")
                        .selectable(false)
                        .ui(ui)
                        .on_hover_text("我自己实现一个字体选择器？真的假的？");
                    let text = egui::RichText::new(&*self.current_font_family)
                        .family(egui::FontFamily::Name(KeyOverlay::FONT_FAMILY_NAME.into()));
                    let combo_box = egui::ComboBox::from_id_salt(ui.next_auto_id())
                        .selected_text(text)
                        .width(0.0);
                    combo_box.show_ui(ui, |ui| {
                        self.font_families.iter().for_each(|font_name| {
                            changed |= ui
                                .selectable_value(
                                    &mut self.current_font_family,
                                    font_name.clone(),
                                    &**font_name,
                                )
                                .changed();
                        });
                    });
                });
            });

            changed
        });

        self.request_reload |= changed;
    }
}

enum GlobalResponse {
    SelectAll,
    CancelSelectAll,
    Write,
    Delete,
    ReadOne,
    WriteOne,
    DeleteOne,
    CreateOne,
    Swap,
    MoveToItsLeft,
    MoveToItsRight,
    CreateMultiple,
}

#[derive(Debug, Default)]
struct GlobalKeyPropertyCheckStates {
    global_operation: bool,
    key_bind: bool,
    key_text: bool,
    font_size: bool,
    position: bool,
    position_x: bool,
    position_y: bool,
    width: bool,
    height: bool,
    thickness: bool,
    frame_color: bool,
    bar_speed: bool,
    pressed_color: bool,
    max_distance: bool,
    key_direction: bool,
    fade_length: bool,
    key_counter: bool,
    key_counter_position: bool,
    key_counter_position_x: bool,
    key_counter_position_y: bool,
    key_counter_size: bool,
    key_counter_color: bool,
}

macro_rules! match_global_key_property_check_states {
    ($self_ident: ident, $macro_op: tt) => {
        let GlobalKeyPropertyCheckStates {
            global_operation: _,
            key_bind,
            key_text,
            font_size,
            position,
            position_x,
            position_y,
            width,
            height,
            thickness,
            frame_color,
            bar_speed,
            pressed_color,
            max_distance,
            key_direction,
            fade_length,
            key_counter,
            key_counter_position,
            key_counter_position_x,
            key_counter_position_y,
            key_counter_size,
            key_counter_color,
        } = &$self_ident.global_key_property_check_states;
        key_bind.then(|| $macro_op!(key_bind));
        key_text.then(|| $macro_op!(key_text, text_color));
        font_size.then(|| $macro_op!(font_size));
        position.then(|| {
            position_x.then(|| $macro_op!(position.x));
            position_y.then(|| $macro_op!(position.y));
        });
        width.then(|| $macro_op!(width));
        height.then(|| $macro_op!(height));
        thickness.then(|| $macro_op!(thickness));
        frame_color.then(|| $macro_op!(frame_color));
        bar_speed.then(|| $macro_op!(bar_speed));
        pressed_color.then(|| $macro_op!(pressed_color));
        max_distance.then(|| $macro_op!(max_distance));
        key_direction.then(|| $macro_op!(key_direction));
        fade_length.then(|| $macro_op!(fade_length));
        key_counter.then(|| $macro_op!(key_counter.0));
        key_counter_position.then(|| {
            key_counter_position_x.then(|| $macro_op!(key_counter.1.position.x));
            key_counter_position_y.then(|| $macro_op!(key_counter.1.position.y));
        });
        key_counter_size.then(|| $macro_op!(key_counter.1.font_size));
        key_counter_color.then(|| $macro_op!(key_counter.1.text_color));
    };
}

#[derive(Debug, Default)]
struct GlobalOperationCache {
    its_index: usize,
    create_count: usize,
    scroll_index: usize,
    need_scrool: bool,
}

struct KeyPropertySettingRow {
    key_properties: Vec<KeyProperty>,
    check_states: Vec<bool>,
    global_key_property: KeyProperty,
    global_key_property_check_states: GlobalKeyPropertyCheckStates,
    global_operation_cache: GlobalOperationCache,
    global_response: Option<GlobalResponse>,
    request_reload: bool,
    key_bind_menu_opened: bool,
    key_binding: Option<Key>,
}

impl KeyPropertySettingRow {
    fn new(setting: &Setting) -> Self {
        Self {
            key_properties: setting.key_properties.clone(),
            check_states: vec![false; setting.key_properties.len()],
            global_key_property: Default::default(),
            global_key_property_check_states: Default::default(),
            global_operation_cache: Default::default(),
            global_response: None,
            request_reload: false,
            key_bind_menu_opened: false,
            key_binding: None,
        }
    }

    fn reload(&mut self, setting: &Setting) {
        self.key_properties = setting.key_properties.clone();
        self.check_states.resize(self.key_properties.len(), false);
    }

    fn update(&mut self, request_reload: &mut bool, keys_receiver: &mut MpscReceiver<KeyMessage>) {
        self.handle_global_response();
        *request_reload |= std::mem::take(&mut self.request_reload);
        if self.key_bind_menu_opened {
            self.key_binding = keys_receiver
                .try_iter()
                .find(|key_message| key_message.is_pressed)
                .map(|key_message| key_message.key);
        } else {
            self.key_binding.take();
        };
    }

    fn handle_global_response(&mut self) {
        self.global_response.take().map(|response| match response {
            GlobalResponse::SelectAll => self.check_states.iter_mut().for_each(|c| *c = true),
            GlobalResponse::CancelSelectAll => {
                self.check_states.iter_mut().for_each(|c| *c = false)
            }
            GlobalResponse::Write => self.handle_global_response_write(),
            GlobalResponse::Delete => self.handle_global_response_delete(),
            GlobalResponse::ReadOne => self.handle_global_response_read_one(),
            GlobalResponse::WriteOne => self.handle_global_response_write_one(),
            GlobalResponse::DeleteOne => self.handle_global_response_delete_one(),
            GlobalResponse::CreateOne => self.handle_global_response_create_one(),
            GlobalResponse::Swap => self.handle_global_response_swap(),
            GlobalResponse::MoveToItsLeft => self.handle_global_response_move_to_its::<false>(),
            GlobalResponse::MoveToItsRight => self.handle_global_response_move_to_its::<true>(),
            GlobalResponse::CreateMultiple => self.handle_global_response_create_multiple(),
        });
    }

    fn handle_global_response_write(&mut self) {
        let iter = self
            .check_states
            .iter()
            .zip(self.key_properties.iter_mut())
            .filter(|&(c, _)| *c);
        iter.for_each(|(_, key_property)| {
            macro_rules! write {
                ($($($token: tt).*),*) => {{
                    $(key_property.$($token).* = self.global_key_property.$($token).*.clone());*
                }};
            }
            match_global_key_property_check_states!(self, write);
        });
        self.request_reload = true;
    }

    fn handle_global_response_delete(&mut self) {
        let new_key_properties = self
            .check_states
            .iter()
            .zip(self.key_properties.drain(..))
            .filter_map(|(c, key_property)| (!c).then(|| key_property))
            .collect();
        self.key_properties = new_key_properties;
        self.request_reload = true;
    }

    fn handle_global_response_read_one(&mut self) {
        let count = self.check_states.iter().filter(|&c| *c).count();
        if count == 1 {
            let index = self.check_states.iter().take_while(|&c| !*c).count();
            let key_property = self.key_properties.get(index).unwrap();
            macro_rules! read_one {
                ($($($token: tt).*),*) => {{
                    $(self.global_key_property.$($token).* = key_property.$($token).*.clone());*
                }};
            }
            match_global_key_property_check_states!(self, read_one);
        } else {
            message_dialog::info("勾选了多个或没有选择").show();
        }
    }

    fn handle_global_response_write_one(&mut self) {
        let count = self.check_states.iter().filter(|&c| *c).count();
        if count == 1 {
            let index = self.check_states.iter().take_while(|&c| !*c).count();
            let key_property = self.key_properties.get_mut(index).unwrap();
            macro_rules! write_one {
                ($($($token: tt).*),*) => {{
                    $(key_property.$($token).* = self.global_key_property.$($token).*.clone());*
                }};
            }
            match_global_key_property_check_states!(self, write_one);
            self.request_reload = true;
        } else {
            message_dialog::info("勾选了多个或没有选择").show();
        }
    }

    fn handle_global_response_delete_one(&mut self) {
        let count = self.check_states.iter().filter(|&c| *c).count();
        if count == 1 {
            let index = self.check_states.iter().take_while(|&c| !*c).count();
            self.key_properties.remove(index);
            self.request_reload = true;
        } else {
            message_dialog::info("勾选了多个或没有选择").show();
        }
    }

    fn handle_global_response_create_one(&mut self) {
        let mut new_key_property = KeyProperty::default();
        macro_rules! create_one {
            ($($($token: tt).*),*) => {{
                $(new_key_property.$($token).* = self.global_key_property.$($token).*.clone());*
            }};
        }
        match_global_key_property_check_states!(self, create_one);
        self.key_properties.push(new_key_property);
        self.request_reload = true;
    }

    fn handle_global_response_swap(&mut self) {
        let count = self.check_states.iter().filter(|&c| *c).count();
        if count == 2 {
            let indexes: Box<_> = self
                .check_states
                .iter()
                .enumerate()
                .filter_map(|(index, &c)| c.then(|| index))
                .collect();
            let (index_l, index_r) = (*indexes.get(0).unwrap(), *indexes.get(1).unwrap());

            // Safety: &mut self; not overlapped
            let key_property_l =
                unsafe { NonNull::from(self.key_properties.get_mut(index_l).unwrap()).as_mut() };
            let key_property_r =
                unsafe { NonNull::from(self.key_properties.get_mut(index_r).unwrap()).as_mut() };

            macro_rules! swap {
                ($($($token: tt).*),*) => {{
                    $(std::mem::swap(
                        &mut key_property_l.$($token).*,
                        &mut key_property_r.$($token).*,
                    ));*
                }};
            }
            match_global_key_property_check_states!(self, swap);
            self.request_reload = true;
        } else {
            message_dialog::info("勾选数量不为2").show();
        }
    }

    fn handle_global_response_move_to_its<const IS_RIGHT: bool>(&mut self) {
        let count = self.check_states.iter().filter(|&c| *c).count();
        if count == 1 {
            let index = self.check_states.iter().take_while(|&c| !*c).count();
            let key_property = self.key_properties.get_mut(index).unwrap();
            let new_key_property = key_property.clone();
            let move_index = self.global_operation_cache.its_index + IS_RIGHT as usize;
            self.key_properties.insert(move_index, new_key_property);
            self.key_properties.remove(index);
            self.request_reload = true;
        } else {
            message_dialog::info("勾选了多个或没有选择").show();
        }
    }

    fn handle_global_response_create_multiple(&mut self) {
        let iter = (0..self.global_operation_cache.create_count)
            .map(|_| {
                let mut new_key_property = KeyProperty::default();
                macro_rules! create_one {
                    ($($($token: tt).*),*) => {{
                        $(new_key_property.$($token).* = self.global_key_property.$($token).*.clone());*
                    }};
                }
                match_global_key_property_check_states!(self, create_one);
                new_key_property
            });
        self.key_properties.extend(iter);
        self.request_reload = true;
    }

    fn show(&mut self, ui: &mut egui::Ui) {
        fn main_layout(
            ui: &mut egui::Ui,
            len: usize,
            add_contents: impl FnOnce(&mut egui::Ui, std::ops::Range<usize>),
        ) {
            let scroll_area = egui::ScrollArea::horizontal();
            let inner = |ui: &mut egui::Ui, range| {
                ui.vertical(|ui| {
                    let layout =
                        egui::Layout::left_to_right(egui::Align::TOP).with_main_wrap(false);
                    ui.allocate_ui_with_layout(Default::default(), layout, |ui| {
                        add_contents(ui, range);
                    });
                    ui.add_space(12.0);
                });
            };
            use crate::utils::egui_scroll_area_show_columns as show_columns;
            show_columns(scroll_area, ui, 280.0, len, inner);
        }

        self.key_bind_menu_opened = false;

        let len = self.key_properties.len();
        main_layout(ui, len, |ui, range| {
            let scroll_index = self.global_operation_cache.scroll_index;

            range.clone().for_each(|index| {
                let frame = egui::Frame::default()
                    .inner_margin(egui::Margin::same(5.0))
                    .stroke(ui.visuals().noninteractive().bg_stroke);
                let inner_response = frame.show(ui, |ui| {
                    egui::Grid::new(ui.next_auto_id())
                        .striped(true)
                        .show(ui, |ui| {
                            self.request_reload |= self.show_column(index, ui);
                        });
                });
                let response = inner_response.response;
                let need_scroll = self.global_operation_cache.need_scrool;
                if need_scroll && scroll_index == index {
                    response.scroll_to_me(Some(egui::Align::Center));
                    self.global_operation_cache.need_scrool = false;
                }
            });

            if self.global_operation_cache.need_scrool {
                let predict_delta = 100.0;
                let animation = egui::style::ScrollAnimation::none();
                if scroll_index < range.start {
                    let diff = (range.start - scroll_index) as f32;
                    ui.scroll_with_delta_animation([predict_delta * diff, 0.0].into(), animation);
                } else if range.end <= scroll_index {
                    let diff = (scroll_index - range.end + 1) as f32;
                    ui.scroll_with_delta_animation([-predict_delta * diff, 0.0].into(), animation);
                }
            }
        });

        ui.separator();

        let frame = egui::Frame::default()
            .inner_margin(egui::Margin::same(5.0))
            .stroke(ui.visuals().noninteractive().bg_stroke);
        frame.show(ui, |ui| {
            let grid = egui::Grid::new(ui.next_auto_id()).striped(true);
            grid.show(ui, |ui| self.global_response = self.show_global_editor(ui));
        });

        ui.separator();
    }

    fn show_column(&mut self, index: usize, ui: &mut egui::Ui) -> bool {
        let key_property = self.key_properties.get_mut(index).unwrap();
        let check_state = self.check_states.get_mut(index).unwrap();

        let mut changed = false;

        // check_state
        grid_new_row!(ui, {
            let text = "勾选或不勾选，这值得思考";
            egui::Label::new("选择:")
                .selectable(false)
                .ui(ui)
                .on_hover_text(text);
            egui::Checkbox::without_text(check_state)
                .ui(ui)
                .on_hover_text(text);
        });

        // index
        grid_new_row!(ui, {
            egui::Label::new("序号:")
                .selectable(false)
                .ui(ui)
                .on_hover_text("序号会影响绘制顺序，序号小的先被绘制。");
            let response = egui::Label::new(index.to_string()).selectable(false).ui(ui);
            if index == 9 {
                response.on_hover_text("baka!");
            }
        });

        // key_bind
        grid_new_row!(ui, {
            egui::Label::new("按键绑定:")
                .selectable(false)
                .ui(ui)
                .on_hover_text(concat!(
                    "点击右边的选项，然后按下一个你想绑定的按键。\n",
                    "当然，你也可以在展开的选项中选一个。"
                ));
            egui::ComboBox::from_id_salt(ui.next_auto_id())
                .selected_text(key_property.key_bind.to_string())
                .width(0.0)
                .show_ui(ui, |ui| {
                    self.key_bind_menu_opened = true;
                    let key_to_scroll = self.key_binding.take().map(|key| {
                        key_property.key_bind = key;
                        changed = true;
                        key
                    });
                    Key::iter().rev().for_each(|key| {
                        let response =
                            ui.selectable_value(&mut key_property.key_bind, key, key.to_string());
                        changed |= response.changed();
                        key_to_scroll.map(|key_to_scroll| {
                            if key_to_scroll == key {
                                response.scroll_to_me(None);
                            }
                        });
                    });
                })
                .response
                .on_hover_text("点击我，然后按下一个按键！");
        });

        // key_text & text_color
        grid_new_row!(ui, {
            egui::Label::new("按键文本:")
                .selectable(false)
                .ui(ui)
                .on_hover_text(concat!(
                    "支持Unicode，不过不能换行。\n",
                    "虽然能实现，但我想它并不需要换行。"
                ));
            ui.horizontal(|ui| {
                changed |= ui
                    .color_edit_button_srgba_unmultiplied(&mut key_property.text_color.0)
                    .changed();
                changed |= egui::TextEdit::singleline(&mut key_property.key_text)
                    .show(ui)
                    .response
                    .changed();
            });
        });

        // font_size
        grid_new_row!(ui, {
            egui::Label::new("字体大小:")
                .selectable(false)
                .ui(ui)
                .on_hover_text("按键文本字体的大小");
            changed |= egui::Slider::new(&mut key_property.font_size, 1.0..=10_000.0)
                .integer()
                .logarithmic(true)
                .drag_value_speed(1.0)
                .ui(ui)
                .changed();
        });

        // position
        grid_new_row!(ui, {
            egui::Label::new("位置:")
                .selectable(false)
                .ui(ui)
                .on_hover_text("按键边框左上角的坐标");
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    egui::Label::new("x:").selectable(false).ui(ui);
                    egui::Slider::new(&mut key_property.position.x, -10_000.0..=10_000.0)
                        .integer()
                        .logarithmic(true)
                        .drag_value_speed(1.0)
                        .ui(ui)
                        .changed()
                        .then(|| changed = true);
                });
                ui.horizontal(|ui| {
                    egui::Label::new("y:").selectable(false).ui(ui);
                    egui::Slider::new(&mut key_property.position.y, -10_000.0..=10_000.0)
                        .integer()
                        .logarithmic(true)
                        .drag_value_speed(1.0)
                        .ui(ui)
                        .changed()
                        .then(|| changed = true);
                });
            });
        });

        // width
        grid_new_row!(ui, {
            egui::Label::new("宽度:")
                .selectable(false)
                .ui(ui)
                .on_hover_text("按键边框的宽度");
            changed |= egui::Slider::new(&mut key_property.width, 3.0..=10_000.0)
                .integer()
                .logarithmic(true)
                .drag_value_speed(1.0)
                .ui(ui)
                .changed();
        });

        // height
        grid_new_row!(ui, {
            egui::Label::new("高度:")
                .selectable(false)
                .ui(ui)
                .on_hover_text("按键边框的高度");
            changed |= egui::Slider::new(&mut key_property.height, 3.0..=10_000.0)
                .integer()
                .logarithmic(true)
                .drag_value_speed(1.0)
                .ui(ui)
                .changed();
        });

        // thickness
        grid_new_row!(ui, {
            egui::Label::new("边框厚度:")
                .selectable(false)
                .ui(ui)
                .on_hover_text("按键边框线的厚度");
            changed |= egui::Slider::new(
                &mut key_property.thickness,
                1.0..=key_property.width.min(key_property.height) / 2.0 - 1.0,
            )
            .integer()
            .logarithmic(true)
            .drag_value_speed(1.0)
            .ui(ui)
            .changed();
        });

        // frame_color
        grid_new_row!(ui, {
            egui::Label::new("边框颜色:")
                .selectable(false)
                .ui(ui)
                .on_hover_text("设置边框的颜色");
            ui.color_edit_button_srgba_unmultiplied(&mut key_property.frame_color.0)
                .changed()
                .then(|| changed = true);
        });

        // bar_speed
        grid_new_row!(ui, {
            egui::Label::new("按键条速度:")
                .selectable(false)
                .ui(ui)
                .on_hover_text("按键条前进的速度");
            changed |= egui::Slider::new(&mut key_property.bar_speed, 1.0..=10_000.0)
                .integer()
                .logarithmic(true)
                .drag_value_speed(1.0)
                .ui(ui)
                .changed();
        });

        // pressed_color
        grid_new_row!(ui, {
            egui::Label::new("按键条颜色:")
                .selectable(false)
                .ui(ui)
                .on_hover_text("设置按键条的颜色");
            changed |= ui
                .color_edit_button_srgba_unmultiplied(&mut key_property.pressed_color.0)
                .changed();
        });

        // max_distance
        grid_new_row!(ui, {
            egui::Label::new("最大距离:")
                .selectable(false)
                .ui(ui)
                .on_hover_text(concat!(
                    "按键条能到达的最大距离。\n",
                    "勾选以启用自定义最大距离，\n",
                    "禁用时按键条在窗口边缘处消失。"
                ));
            ui.horizontal(|ui| {
                changed |= egui::Checkbox::without_text(&mut key_property.max_distance.0)
                    .ui(ui)
                    .changed();
                let slider = egui::Slider::new(&mut key_property.max_distance.1, 1.0..=10_000.0)
                    .integer()
                    .logarithmic(true)
                    .drag_value_speed(1.0);
                changed |= ui
                    .add_enabled(key_property.max_distance.0, slider)
                    .changed();
            });
        });

        // key_direction
        grid_new_row!(ui, {
            egui::Label::new("按键方向:")
                .selectable(false)
                .ui(ui)
                .on_hover_text("按键条的前进方向");
            let map = |direction: KeyDirection| match direction {
                KeyDirection::Up => "上",
                KeyDirection::Down => "下",
                KeyDirection::Left => "左",
                KeyDirection::Right => "右",
            };
            egui::ComboBox::from_id_salt(ui.next_auto_id())
                .selected_text(map(key_property.key_direction))
                .width(0.0)
                .show_ui(ui, |ui| {
                    use KeyDirection::*;
                    [(Up, "上"), (Down, "下"), (Left, "左"), (Right, "右")]
                        .into_iter()
                        .for_each(|(direction, name)| {
                            changed |= ui
                                .selectable_value(&mut key_property.key_direction, direction, name)
                                .changed();
                        });
                });
        });

        // fade_length
        grid_new_row!(ui, {
            egui::Label::new("渐隐距离:")
                .selectable(false)
                .ui(ui)
                .on_hover_text(concat!("渐隐效果的距离。\n", "勾选以启用渐隐效果。"));
            ui.horizontal(|ui| {
                changed |= egui::Checkbox::without_text(&mut key_property.fade_length.0)
                    .ui(ui)
                    .changed();
                let slider = egui::Slider::new(&mut key_property.fade_length.1, 1.0..=10_000.0)
                    .integer()
                    .logarithmic(true)
                    .drag_value_speed(1.0);
                changed |= ui.add_enabled(key_property.fade_length.0, slider).changed();
            });
        });

        // key_counter enable
        grid_new_row!(ui, {
            egui::Label::new("启用计数器:")
                .selectable(false)
                .ui(ui)
                .on_hover_text("勾选以启用计数器");
            changed |= egui::Checkbox::without_text(&mut key_property.key_counter.0)
                .ui(ui)
                .changed();
        });

        // key_counter position
        grid_new_row!(ui, {
            egui::Label::new("计数器位置:")
                .selectable(false)
                .ui(ui)
                .on_hover_text("计数器中心点相对于按键边框中心点的位置");
            fn layout(ui: &mut egui::Ui, enable: bool, add_contents: impl FnOnce(&mut egui::Ui)) {
                ui.add_enabled_ui(enable, |ui| ui.vertical(add_contents));
            }
            layout(ui, key_property.key_counter.0, |ui| {
                ui.horizontal(|ui| {
                    egui::Label::new("x:").selectable(false).ui(ui);
                    changed |= egui::Slider::new(
                        &mut key_property.key_counter.1.position.x,
                        -10_000.0..=10_000.0,
                    )
                    .integer()
                    .logarithmic(true)
                    .drag_value_speed(1.0)
                    .ui(ui)
                    .changed();
                });
                ui.horizontal(|ui| {
                    egui::Label::new("y:").selectable(false).ui(ui);
                    changed |= egui::Slider::new(
                        &mut key_property.key_counter.1.position.y,
                        -10_000.0..=10_000.0,
                    )
                    .integer()
                    .logarithmic(true)
                    .drag_value_speed(1.0)
                    .ui(ui)
                    .changed();
                });
            });
        });

        // key_counter size
        grid_new_row!(ui, {
            egui::Label::new("计数器大小:")
                .selectable(false)
                .ui(ui)
                .on_hover_text("计数器字体的大小");
            let slider =
                egui::Slider::new(&mut key_property.key_counter.1.font_size, 1.0..=10_000.0)
                    .integer()
                    .logarithmic(true)
                    .drag_value_speed(1.0);
            changed |= ui.add_enabled(key_property.key_counter.0, slider).changed();
        });

        // key_counter color
        grid_new_row!(ui, {
            egui::Label::new("计数器颜色:")
                .selectable(false)
                .ui(ui)
                .on_hover_text("计数器文本的颜色");
            ui.add_enabled_ui(key_property.key_counter.0, |ui| {
                changed |= ui
                    .color_edit_button_srgba_unmultiplied(
                        &mut key_property.key_counter.1.text_color.0,
                    )
                    .changed();
            });
        });

        changed
    }

    fn show_global_editor(&mut self, ui: &mut egui::Ui) -> Option<GlobalResponse> {
        let key_property = &mut self.global_key_property;
        let GlobalKeyPropertyCheckStates {
            global_operation,
            key_bind,
            key_text,
            font_size,
            position,
            position_x,
            position_y,
            width,
            height,
            thickness,
            frame_color,
            bar_speed,
            pressed_color,
            max_distance,
            key_direction,
            fade_length,
            key_counter,
            key_counter_position,
            key_counter_position_x,
            key_counter_position_y,
            key_counter_size,
            key_counter_color,
        } = &mut self.global_key_property_check_states;

        let mut response = None;

        let mut set_response = |r| response = Some(r);

        // global operation
        grid_new_row!(ui, {
            egui::Checkbox::without_text(global_operation)
                .ui(ui)
                .on_hover_text("勾选所有属性，或者取消勾选")
                .clicked()
                .then(|| {
                    let v = *global_operation;
                    *key_bind = v;
                    *key_text = v;
                    *font_size = v;
                    *position = v;
                    *position_x = v;
                    *position_y = v;
                    *width = v;
                    *height = v;
                    *thickness = v;
                    *frame_color = v;
                    *bar_speed = v;
                    *pressed_color = v;
                    *max_distance = v;
                    *key_direction = v;
                    *fade_length = v;
                    *key_counter = v;
                    *key_counter_position = v;
                    *key_counter_position_x = v;
                    *key_counter_position_y = v;
                    *key_counter_size = v;
                    *key_counter_color = v;
                });
            egui::Label::new("操作:")
                .selectable(false)
                .ui(ui)
                .on_hover_text("勾选相关操作");
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    egui::Label::new("快捷浏览: ")
                        .selectable(false)
                        .ui(ui)
                        .on_hover_text("用它可以快速地定位");
                    let slider = egui::Slider::new(
                        &mut self.global_operation_cache.scroll_index,
                        0..=self.key_properties.len() - 1,
                    )
                    .drag_value_speed(1.0);
                    ui.add_enabled(!self.key_properties.is_empty(), slider)
                        .changed()
                        .then(|| self.global_operation_cache.need_scrool = true);
                });
                let grid_top = egui::Grid::new(ui.next_auto_id()).min_col_width(0.0);
                grid_top.show(ui, |ui| {
                    grid_new_row!(ui, {
                        ui.button("全选")
                            .on_hover_text("全部勾选")
                            .clicked()
                            .then(|| set_response(GlobalResponse::SelectAll));
                        ui.button("取消全选")
                            .on_hover_text("全部取消勾选")
                            .clicked()
                            .then(|| set_response(GlobalResponse::CancelSelectAll));
                        ui.button("写入")
                            .on_hover_text("将被勾选的属性写入到被勾选的项目中")
                            .clicked()
                            .then(|| set_response(GlobalResponse::Write));
                        ui.button("删除")
                            .on_hover_text("删除所选项目")
                            .clicked()
                            .then(|| set_response(GlobalResponse::Delete));
                    });

                    grid_new_row!(ui, {
                        ui.button("读取单项")
                            .on_hover_text("读取单个项目的被勾选的属性")
                            .clicked()
                            .then(|| set_response(GlobalResponse::ReadOne));
                        ui.button("写入单项")
                            .on_hover_text("将给勾选的属性写入到单个项目")
                            .clicked()
                            .then(|| set_response(GlobalResponse::WriteOne));
                        ui.button("删除单项")
                            .on_hover_text("删除单个项目")
                            .clicked()
                            .then(|| set_response(GlobalResponse::DeleteOne));
                        ui.button("新建单项")
                            .on_hover_text("使用被勾选的属性创建新的项目，未勾选的使用缺省值")
                            .clicked()
                            .then(|| set_response(GlobalResponse::CreateOne));
                    });

                    grid_new_row!(ui, {
                        ui.button("交换")
                            .on_hover_text("交换两个项目被勾选的属性")
                            .clicked()
                            .then(|| set_response(GlobalResponse::Swap));
                    });
                });

                let grid_bottom = egui::Grid::new(ui.next_auto_id()).min_col_width(0.0);
                grid_bottom.show(ui, |ui| {
                    grid_new_row!(ui, {
                        ui.vertical(|ui| {
                            ui.set_min_width(100.0);
                            ui.button("移动到它的左侧")
                                .on_hover_text("将所选项目移动到它的左侧")
                                .clicked()
                                .then(|| set_response(GlobalResponse::MoveToItsLeft));
                            ui.button("移动到它的右侧")
                                .on_hover_text("将所选项目移动到它的右侧")
                                .clicked()
                                .then(|| set_response(GlobalResponse::MoveToItsRight));
                        });
                        ui.horizontal(|ui| {
                            egui::Label::new("它的序号:").selectable(false).ui(ui);
                            egui::Slider::new(
                                &mut self.global_operation_cache.its_index,
                                0..=self.key_properties.len() - 1,
                            )
                            .drag_value_speed(1.0)
                            .ui(ui);
                        });
                    });

                    grid_new_row!(ui, {
                        ui.button("新建多项")
                            .on_hover_text("一次性创建多个！")
                            .clicked()
                            .then(|| set_response(GlobalResponse::CreateMultiple));
                        let response = egui::Slider::new(
                            &mut self.global_operation_cache.create_count,
                            0..=1_000,
                        )
                        .logarithmic(true)
                        .drag_value_speed(1.0)
                        .ui(ui);
                        let count = self.global_operation_cache.create_count;
                        if count == 0 {
                            response.on_hover_text("0个？");
                        } else if count == 1 {
                            response.on_hover_text("如同\"新建单项\"那样...");
                        } else if count >= 100 && count < 1000 {
                            response.on_hover_text("你要这么多干什么啦");
                        } else if count == 1000 {
                            response.on_hover_text("最多给你一次创建一千个，再多也没必要吧？");
                        }
                    });
                });
            });
        });

        // key_bind
        grid_new_row!(ui, {
            egui::Checkbox::without_text(key_bind)
                .ui(ui)
                .on_hover_text("勾选以影响该属性");
            egui::Label::new("按键绑定:")
                .selectable(false)
                .ui(ui)
                .on_hover_text(concat!(
                    "点击右边的选项，然后按下一个你想绑定的按键。\n",
                    "当然，你也可以在展开的选项中选一个。"
                ));
            egui::ComboBox::from_id_salt(ui.next_auto_id())
                .selected_text(key_property.key_bind.to_string())
                .width(0.0)
                .show_ui(ui, |ui| {
                    self.key_bind_menu_opened = true;
                    let key_to_scroll = self.key_binding.take().map(|key| {
                        key_property.key_bind = key;
                        key
                    });
                    Key::iter().for_each(|key| {
                        let response =
                            ui.selectable_value(&mut key_property.key_bind, key, key.to_string());
                        key_to_scroll.map(|key_to_scroll| {
                            if key_to_scroll == key {
                                response.scroll_to_me(None);
                            }
                        });
                    });
                })
                .response
                .on_hover_text("点击我，然后按下一个按键！");
        });

        // key_text & text_color
        grid_new_row!(ui, {
            egui::Checkbox::without_text(key_text)
                .ui(ui)
                .on_hover_text("勾选以影响该属性");
            egui::Label::new("按键文本:")
                .selectable(false)
                .ui(ui)
                .on_hover_text(concat!(
                    "支持Unicode，不过不能换行。\n",
                    "虽然能实现，但我想它并不需要换行。"
                ));
            ui.horizontal(|ui| {
                ui.color_edit_button_srgba_unmultiplied(&mut key_property.text_color.0);
                egui::TextEdit::singleline(&mut key_property.key_text).show(ui);
            });
        });

        // font_size
        grid_new_row!(ui, {
            egui::Checkbox::without_text(font_size)
                .ui(ui)
                .on_hover_text("勾选以影响该属性");
            egui::Label::new("字体大小:")
                .selectable(false)
                .ui(ui)
                .on_hover_text("按键文本字体的大小");
            egui::Slider::new(&mut key_property.font_size, 1.0..=10_000.0)
                .integer()
                .logarithmic(true)
                .drag_value_speed(1.0)
                .ui(ui);
        });

        // position
        grid_new_row!(ui, {
            egui::Checkbox::without_text(position)
                .ui(ui)
                .on_hover_text("勾选以影响该属性");
            egui::Label::new("位置:")
                .selectable(false)
                .ui(ui)
                .on_hover_text("按键边框左上角的坐标");
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    egui::Checkbox::without_text(position_x)
                        .ui(ui)
                        .on_hover_text("勾选以影响该属性");
                    egui::Label::new("x:").selectable(false).ui(ui);
                    egui::Slider::new(&mut key_property.position.x, -10_000.0..=10_000.0)
                        .integer()
                        .logarithmic(true)
                        .drag_value_speed(1.0)
                        .ui(ui);
                });
                ui.horizontal(|ui| {
                    egui::Checkbox::without_text(position_y)
                        .ui(ui)
                        .on_hover_text("勾选以影响该属性");
                    egui::Label::new("y:").selectable(false).ui(ui);
                    egui::Slider::new(&mut key_property.position.y, -10_000.0..=10_000.0)
                        .integer()
                        .logarithmic(true)
                        .drag_value_speed(1.0)
                        .ui(ui);
                });
            });
        });

        // width
        grid_new_row!(ui, {
            egui::Checkbox::without_text(width)
                .ui(ui)
                .on_hover_text("勾选以影响该属性");
            egui::Label::new("宽度:")
                .selectable(false)
                .ui(ui)
                .on_hover_text("按键边框的宽度");
            egui::Slider::new(&mut key_property.width, 3.0..=10_000.0)
                .integer()
                .logarithmic(true)
                .drag_value_speed(1.0)
                .ui(ui);
        });

        // height
        grid_new_row!(ui, {
            egui::Checkbox::without_text(height)
                .ui(ui)
                .on_hover_text("勾选以影响该属性");
            egui::Label::new("高度:")
                .selectable(false)
                .ui(ui)
                .on_hover_text("按键边框的高度");
            egui::Slider::new(&mut key_property.height, 3.0..=10_000.0)
                .integer()
                .logarithmic(true)
                .drag_value_speed(1.0)
                .ui(ui);
        });

        // thickness
        grid_new_row!(ui, {
            egui::Checkbox::without_text(thickness)
                .ui(ui)
                .on_hover_text("勾选以影响该属性");
            egui::Label::new("边框厚度:")
                .selectable(false)
                .ui(ui)
                .on_hover_text("按键边框线的厚度");
            egui::Slider::new(
                &mut key_property.thickness,
                1.0..=key_property.width.min(key_property.height) / 2.0 - 1.0,
            )
            .integer()
            .logarithmic(true)
            .drag_value_speed(1.0)
            .ui(ui);
        });

        // frame_color
        grid_new_row!(ui, {
            egui::Checkbox::without_text(frame_color)
                .ui(ui)
                .on_hover_text("勾选以影响该属性");
            egui::Label::new("边框颜色:")
                .selectable(false)
                .ui(ui)
                .on_hover_text("设置边框的颜色");
            ui.color_edit_button_srgba_unmultiplied(&mut key_property.frame_color.0);
        });

        // bar_speed
        grid_new_row!(ui, {
            egui::Checkbox::without_text(bar_speed)
                .ui(ui)
                .on_hover_text("勾选以影响该属性");
            egui::Label::new("按键条速度:")
                .selectable(false)
                .ui(ui)
                .on_hover_text("按键条前进的速度");
            egui::Slider::new(&mut key_property.bar_speed, 1.0..=10_000.0)
                .integer()
                .logarithmic(true)
                .drag_value_speed(1.0)
                .ui(ui);
        });

        // pressed_color
        grid_new_row!(ui, {
            egui::Checkbox::without_text(pressed_color)
                .ui(ui)
                .on_hover_text("勾选以影响该属性");
            egui::Label::new("按键条颜色:")
                .selectable(false)
                .ui(ui)
                .on_hover_text("设置按键条的颜色");
            ui.color_edit_button_srgba_unmultiplied(&mut key_property.pressed_color.0);
        });

        // max_distance
        grid_new_row!(ui, {
            egui::Checkbox::without_text(max_distance)
                .ui(ui)
                .on_hover_text("勾选以影响该属性");
            egui::Label::new("最大距离:")
                .selectable(false)
                .ui(ui)
                .on_hover_text(concat!(
                    "按键条能到达的最大距离。\n",
                    "勾选以启用自定义最大距离，\n",
                    "禁用时按键条在窗口边缘处消失。"
                ));
            ui.horizontal(|ui| {
                egui::Checkbox::without_text(&mut key_property.max_distance.0).ui(ui);
                egui::Slider::new(&mut key_property.max_distance.1, 1.0..=10_000.0)
                    .integer()
                    .logarithmic(true)
                    .drag_value_speed(1.0)
                    .ui(ui);
            });
        });

        // key_direction
        grid_new_row!(ui, {
            egui::Checkbox::without_text(key_direction)
                .ui(ui)
                .on_hover_text("勾选以影响该属性");
            egui::Label::new("按键方向:")
                .selectable(false)
                .ui(ui)
                .on_hover_text("按键条的前进方向");
            let map = |direction: KeyDirection| match direction {
                KeyDirection::Up => "上",
                KeyDirection::Down => "下",
                KeyDirection::Left => "左",
                KeyDirection::Right => "右",
            };
            egui::ComboBox::from_id_salt(ui.next_auto_id())
                .selected_text(map(key_property.key_direction))
                .width(0.0)
                .show_ui(ui, |ui| {
                    use KeyDirection::*;
                    [(Up, "上"), (Down, "下"), (Left, "左"), (Right, "右")]
                        .into_iter()
                        .for_each(|(direction, name)| {
                            ui.selectable_value(&mut key_property.key_direction, direction, name);
                        });
                });
        });

        // fade_length
        grid_new_row!(ui, {
            egui::Checkbox::without_text(fade_length)
                .ui(ui)
                .on_hover_text("勾选以影响该属性");
            egui::Label::new("渐隐距离:")
                .selectable(false)
                .ui(ui)
                .on_hover_text(concat!("渐隐效果的距离。\n", "勾选以启用渐隐效果。"));
            ui.horizontal(|ui| {
                egui::Checkbox::without_text(&mut key_property.fade_length.0).ui(ui);
                egui::Slider::new(&mut key_property.fade_length.1, 1.0..=10_000.0)
                    .integer()
                    .logarithmic(true)
                    .drag_value_speed(1.0)
                    .ui(ui);
            });
        });

        // key_counter enable
        grid_new_row!(ui, {
            egui::Checkbox::without_text(key_counter)
                .ui(ui)
                .on_hover_text("勾选以影响该属性");
            egui::Label::new("启用计数器:")
                .selectable(false)
                .ui(ui)
                .on_hover_text("勾选以启用计数器");
            egui::Checkbox::without_text(&mut key_property.key_counter.0).ui(ui);
        });

        // key_counter position
        grid_new_row!(ui, {
            egui::Checkbox::without_text(key_counter_position)
                .ui(ui)
                .on_hover_text("勾选以影响该属性");
            egui::Label::new("计数器位置:")
                .selectable(false)
                .ui(ui)
                .on_hover_text("计数器中心点相对于按键边框中心点的位置");
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    egui::Checkbox::without_text(key_counter_position_x)
                        .ui(ui)
                        .on_hover_text("勾选以影响该属性");
                    egui::Label::new("x:").selectable(false).ui(ui);
                    egui::Slider::new(
                        &mut key_property.key_counter.1.position.x,
                        -10_000.0..=10_000.0,
                    )
                    .integer()
                    .logarithmic(true)
                    .drag_value_speed(1.0)
                    .ui(ui);
                });
                ui.horizontal(|ui| {
                    egui::Checkbox::without_text(key_counter_position_y)
                        .ui(ui)
                        .on_hover_text("勾选以影响该属性");
                    egui::Label::new("y:").selectable(false).ui(ui);
                    egui::Slider::new(
                        &mut key_property.key_counter.1.position.y,
                        -10_000.0..=10_000.0,
                    )
                    .integer()
                    .logarithmic(true)
                    .drag_value_speed(1.0)
                    .ui(ui);
                });
            });
        });

        // key_counter size
        grid_new_row!(ui, {
            egui::Checkbox::without_text(key_counter_size)
                .ui(ui)
                .on_hover_text("勾选以影响该属性");
            egui::Label::new("计数器大小:")
                .selectable(false)
                .ui(ui)
                .on_hover_text("计数器字体的大小");
            egui::Slider::new(&mut key_property.key_counter.1.font_size, 1.0..=10_000.0)
                .integer()
                .logarithmic(true)
                .drag_value_speed(1.0)
                .ui(ui);
        });

        // key_counter color
        grid_new_row!(ui, {
            egui::Checkbox::without_text(key_counter_color)
                .ui(ui)
                .on_hover_text("勾选以影响该属性");
            egui::Label::new("计数器颜色:")
                .selectable(false)
                .ui(ui)
                .on_hover_text("计数器文本的颜色");
            ui.color_edit_button_srgba_unmultiplied(&mut key_property.key_counter.1.text_color.0);
        });

        response
    }
}
