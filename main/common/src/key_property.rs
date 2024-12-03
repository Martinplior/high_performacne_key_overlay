#![allow(dead_code)]

use serde::{Deserialize, Serialize};

use crate::{key::Key, ucolor32::UColor32};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyCounterProperty {
    /// relative to the center of key's frame
    pub position: egui::Pos2,
    pub font_size: f32,
    pub text_color: UColor32,
}

impl Default for KeyCounterProperty {
    fn default() -> Self {
        Self {
            position: egui::pos2(0.0, KeyProperty::DEFAULT_HEIGHT),
            font_size: KeyProperty::DEFAULT_FONT_SIZE,
            text_color: KeyProperty::DEFAULT_TEXT_COLOR,
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyDirection {
    #[default]
    Up,
    Down,
    Left,
    Right,
}

impl KeyCounterProperty {
    pub fn with_position(mut self, position: egui::Pos2) -> Self {
        self.position = position;
        self
    }

    pub fn with_font_size(mut self, font_size: f32) -> Self {
        self.font_size = font_size;
        self
    }

    pub fn with_text_color(mut self, text_color: UColor32) -> Self {
        self.text_color = text_color;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyProperty {
    pub key_bind: Key,
    pub key_text: String,
    /// top-left corner
    pub position: egui::Pos2,
    pub width: f32,
    pub height: f32,
    pub thickness: f32,
    pub font_size: f32,
    pub bar_speed: f32,
    pub max_distance: (bool, f32),
    pub text_color: UColor32,
    pub frame_color: UColor32,
    pub pressed_color: UColor32,
    pub key_direction: KeyDirection,
    pub fade_length: (bool, f32),
    pub key_counter: (bool, KeyCounterProperty),
}

impl Default for KeyProperty {
    fn default() -> Self {
        Self {
            key_bind: Default::default(),
            key_text: "".into(),
            position: egui::Pos2::default(),
            width: Self::DEFAULT_WIDTH,
            height: Self::DEFAULT_HEIGHT,
            thickness: Self::DEFAULT_THICKNESS,
            font_size: Self::DEFAULT_FONT_SIZE,
            bar_speed: Self::DEFAULT_BAR_SPEED,
            max_distance: (false, 200.0),
            text_color: Self::DEFAULT_TEXT_COLOR,
            frame_color: Self::DEFAULT_FRAME_COLOR,
            pressed_color: Self::DEFAULT_PRESSED_COLOR,
            fade_length: Self::DEFAULT_FADE_LENGTH,
            key_direction: Default::default(),
            key_counter: (false, KeyCounterProperty::default()),
        }
    }
}

impl KeyProperty {
    pub const DEFAULT_WIDTH: f32 = 20.0;
    pub const DEFAULT_HEIGHT: f32 = 20.0;
    pub const DEFAULT_THICKNESS: f32 = 3.0;
    pub const DEFAULT_FONT_SIZE: f32 = 12.0;
    pub const DEFAULT_BAR_SPEED: f32 = 500.0;
    pub const DEFAULT_TEXT_COLOR: UColor32 = UColor32::WHITE;
    pub const DEFAULT_FRAME_COLOR: UColor32 = UColor32::WHITE;
    /// same as `Color32::from_white_alpha(0x80)`
    pub const DEFAULT_PRESSED_COLOR: UColor32 = UColor32::WHITE.with_a(128);

    pub const DEFAULT_FADE_LENGTH: (bool, f32) = (true, 50.0);

    pub fn with_key_bind(mut self, key_bind: Key) -> Self {
        self.key_bind = key_bind;
        self
    }

    pub fn with_key_text(mut self, key_text: String) -> Self {
        self.key_text = key_text;
        self
    }

    pub fn with_position(mut self, position: egui::Pos2) -> Self {
        self.position = position;
        self
    }

    pub fn with_width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn with_height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn with_thickness(mut self, thickness: f32) -> Self {
        self.thickness = thickness;
        self
    }

    pub fn with_font_size(mut self, font_size: f32) -> Self {
        self.font_size = font_size;
        self
    }

    pub fn with_bar_speed(mut self, bar_speed: f32) -> Self {
        self.bar_speed = bar_speed;
        self
    }

    pub fn with_max_distance(mut self, max_distance: Option<f32>) -> Self {
        if let Some(max_distance) = max_distance {
            self.max_distance = (true, max_distance);
        } else {
            self.max_distance.0 = false;
        }
        self
    }

    pub fn with_text_color(mut self, text_color: UColor32) -> Self {
        self.text_color = text_color;
        self
    }

    pub fn with_frame_color(mut self, frame_color: UColor32) -> Self {
        self.frame_color = frame_color;
        self
    }

    pub fn with_pressed_color(mut self, pressed_color: UColor32) -> Self {
        self.pressed_color = pressed_color;
        self
    }

    pub fn with_fade_length(mut self, fade_length: Option<f32>) -> Self {
        if let Some(fade_length) = fade_length {
            self.fade_length = (true, fade_length);
        } else {
            self.fade_length.0 = false
        }
        self
    }

    pub fn with_key_direction(mut self, key_direction: KeyDirection) -> Self {
        self.key_direction = key_direction;
        self
    }

    pub fn with_key_counter(mut self, key_counter: Option<KeyCounterProperty>) -> Self {
        if let Some(key_counter) = key_counter {
            self.key_counter = (true, key_counter);
        } else {
            self.key_counter.0 = false;
        }
        self
    }
}
