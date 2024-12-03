use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use egui::{Color32, FontId, Rangef, Rounding, TextureId};

use crate::{
    key_bar::KeyBar,
    key_property::{KeyDirection, KeyProperty},
    setting::WindowSetting,
};

#[derive(Debug)]
pub struct KeyDrawer {
    bar_queue: VecDeque<KeyBar>,
    key_text_color: Color32,
    frame_color: Color32,
    pressed_color: Color32,
    key_counter_color: Color32,
    max_bar_duration: Duration,
    count: u32,
    pub begin_hold_instant: Option<Instant>,
}

impl KeyDrawer {
    pub fn new(window_setting: &WindowSetting, bar_speed: f32, key_property: &KeyProperty) -> Self {
        let max_bar_duration = Duration::from_secs_f32(
            if let (true, max_distantce) = key_property.max_distance {
                max_distantce
            } else {
                match key_property.key_direction {
                    KeyDirection::Up => key_property.position.y,
                    KeyDirection::Down => {
                        window_setting.height - key_property.position.y - key_property.height
                    }
                    KeyDirection::Left => key_property.position.x,
                    KeyDirection::Right => {
                        window_setting.width - key_property.position.x - key_property.width
                    }
                }
                .max(0.0)
            } / bar_speed,
        );
        Self {
            bar_queue: VecDeque::with_capacity(64),
            key_text_color: key_property.text_color.into(),
            frame_color: key_property.frame_color.into(),
            pressed_color: key_property.pressed_color.into(),
            key_counter_color: key_property.key_counter.1.text_color.into(),
            max_bar_duration,
            count: 0,
            begin_hold_instant: None,
        }
    }

    pub fn increase_count(&mut self) {
        self.count = self.count.wrapping_add(1);
    }

    pub fn need_repaint(&self) -> bool {
        !self.bar_queue.is_empty() || self.begin_hold_instant.is_some()
    }

    pub fn add_bar(&mut self, bar: KeyBar) {
        self.bar_queue.push_back(bar);
    }

    pub fn remove_outer_bar(&mut self, instant_now: Instant) {
        let dead_line = instant_now - self.max_bar_duration;
        while let Some(bar) = self.bar_queue.front() {
            if bar.release_instant < dead_line {
                self.bar_queue.pop_front();
            } else {
                break;
            }
        }
    }

    pub fn draw_on(
        &self,
        painter: &egui::Painter,
        key_property: &KeyProperty,
        instant_now: Instant,
        font_family: &egui::FontFamily,
        fade_mask_id: [TextureId; 4],
    ) {
        let clip_rect = painter.clip_rect();

        let egui::Rect { min: clip_min, .. } = clip_rect;

        let pos_remap = |pos: egui::Pos2| clip_min + pos.to_vec2();

        let key_position = pos_remap(key_property.position);

        let up_down_x_range = Rangef::new(key_position.x, key_position.x + key_property.width);
        let left_right_y_range = Rangef::new(key_position.y, key_position.y + key_property.height);

        let bar_rect = |head: f32, tail: f32| match key_property.key_direction {
            KeyDirection::Up => {
                let bar_pos_remap = |pos: f32| key_position.y + pos;
                let (head, tail) = (bar_pos_remap(-head), bar_pos_remap(-tail));
                egui::Rect::from_x_y_ranges(up_down_x_range, head..=tail)
            }
            KeyDirection::Down => {
                let bar_pos_remap = |pos: f32| key_position.y + key_property.height + pos;
                let (head, tail) = (bar_pos_remap(head), bar_pos_remap(tail));
                egui::Rect::from_x_y_ranges(up_down_x_range, tail..=head)
            }
            KeyDirection::Left => {
                let bar_pos_remap = |pos: f32| key_position.x + pos;
                let (head, tail) = (bar_pos_remap(-head), bar_pos_remap(-tail));
                egui::Rect::from_x_y_ranges(head..=tail, left_right_y_range)
            }
            KeyDirection::Right => {
                let bar_pos_remap = |pos: f32| key_position.x + key_property.width + pos;
                let (head, tail) = (bar_pos_remap(head), bar_pos_remap(tail));
                egui::Rect::from_x_y_ranges(tail..=head, left_right_y_range)
            }
        };

        let direction_clip_rect = if let (true, max_distance) = key_property.max_distance {
            let rect = match key_property.key_direction {
                KeyDirection::Up => clip_rect.with_min_y(key_position.y - max_distance),
                KeyDirection::Down => {
                    clip_rect.with_max_y(key_position.y + key_property.height + max_distance)
                }
                KeyDirection::Left => clip_rect.with_min_x(key_position.x - max_distance),
                KeyDirection::Right => {
                    clip_rect.with_max_x(key_position.x + key_property.width + max_distance)
                }
            };
            Some(rect)
        } else {
            None
        };

        // draw bars
        let bars_painter = direction_clip_rect
            .map_or_else(|| painter.clone(), |clip| painter.with_clip_rect(clip));
        let bar_shapes_iter = self.bar_queue.iter().map(|bar| {
            let (head, mut tail) = bar.into_range(instant_now, key_property.bar_speed);
            // head - tail >= 1.0
            tail = tail.min(head - 1.0);
            let rect = bar_rect(head, tail);
            egui::Shape::Rect(epaint::RectShape::filled(
                rect,
                Rounding::ZERO,
                self.pressed_color,
            ))
        });
        bars_painter.extend(bar_shapes_iter);
        self.begin_hold_instant.map(|current_pressed_instant| {
            let head = KeyBar::compute_pos(
                instant_now - current_pressed_instant,
                key_property.bar_speed,
            );
            let rect = bar_rect(head, 0.0);
            bars_painter.rect_filled(rect, Rounding::ZERO, key_property.pressed_color);
        });
        if let (true, fade_length) = key_property.fade_length {
            let egui::Rect {
                min: clip_min,
                max: clip_max,
            } = bars_painter.clip_rect();
            let [up_mask, down_mask, left_mask, right_mask] = fade_mask_id;
            let direction = key_property.key_direction;
            let (rect, fill_texture_id) = match direction {
                KeyDirection::Up => (
                    egui::Rect::from_x_y_ranges(
                        up_down_x_range,
                        clip_min.y..=clip_min.y + fade_length,
                    ),
                    up_mask,
                ),
                KeyDirection::Down => (
                    egui::Rect::from_x_y_ranges(
                        up_down_x_range,
                        clip_max.y - fade_length..=clip_max.y,
                    ),
                    down_mask,
                ),
                KeyDirection::Left => (
                    egui::Rect::from_x_y_ranges(
                        clip_min.x..=clip_min.x + fade_length,
                        left_right_y_range,
                    ),
                    left_mask,
                ),
                KeyDirection::Right => (
                    egui::Rect::from_x_y_ranges(
                        clip_max.x - fade_length..=clip_max.x,
                        left_right_y_range,
                    ),
                    right_mask,
                ),
            };
            let shape = epaint::RectShape {
                rect,
                rounding: Rounding::ZERO,
                fill: Color32::WHITE,
                stroke: egui::Stroke::NONE,
                blur_width: 0.0,
                fill_texture_id,
                uv: egui::Rect::from_min_max([0.0, 0.0].into(), [1.0, 1.0].into()),
            };
            bars_painter.add(egui::Shape::Rect(shape));
        }

        // draw frame
        let rect = egui::Rect::from_min_size(
            key_position,
            [key_property.width, key_property.height].into(),
        )
        .shrink(key_property.thickness);
        painter.rect(
            rect,
            Rounding::ZERO,
            if self.begin_hold_instant.is_some() {
                self.pressed_color
            } else {
                Color32::TRANSPARENT
            },
            egui::Stroke::new(key_property.thickness, self.frame_color),
        );

        // draw key text
        let key_text_position =
            key_position + egui::vec2(key_property.width / 2.0, key_property.height / 2.0);
        painter.text(
            key_text_position,
            egui::Align2::CENTER_CENTER,
            &key_property.key_text,
            FontId::new(key_property.font_size, font_family.clone()),
            self.key_text_color,
        );

        // draw counter text
        if let (true, counter) = &key_property.key_counter {
            painter.text(
                key_text_position + counter.position.to_vec2(),
                egui::Align2::CENTER_CENTER,
                self.count,
                FontId::new(counter.font_size, font_family.clone()),
                self.key_counter_color,
            );
        }
    }
}
