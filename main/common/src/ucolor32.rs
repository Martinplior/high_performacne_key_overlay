#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// unmultiplied version of [`egui::Color32`]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UColor32(pub [u8; 4]);

impl UColor32 {
    pub const TRANSPARENT: Self = Self::new(0, 0, 0, 0);
    pub const BLACK: Self = Self::from_rgb(0, 0, 0);
    pub const DARK_GRAY: Self = Self::from_rgb(96, 96, 96);
    pub const GRAY: Self = Self::from_rgb(160, 160, 160);
    pub const LIGHT_GRAY: Self = Self::from_rgb(220, 220, 220);
    pub const WHITE: Self = Self::from_rgb(255, 255, 255);

    pub const BROWN: Self = Self::from_rgb(165, 42, 42);
    pub const DARK_RED: Self = Self::from_rgb(0x8B, 0, 0);
    pub const RED: Self = Self::from_rgb(255, 0, 0);
    pub const LIGHT_RED: Self = Self::from_rgb(255, 128, 128);

    pub const YELLOW: Self = Self::from_rgb(255, 255, 0);
    pub const ORANGE: Self = Self::from_rgb(255, 165, 0);
    pub const LIGHT_YELLOW: Self = Self::from_rgb(255, 255, 0xE0);
    pub const KHAKI: Self = Self::from_rgb(240, 230, 140);

    pub const DARK_GREEN: Self = Self::from_rgb(0, 0x64, 0);
    pub const GREEN: Self = Self::from_rgb(0, 255, 0);
    pub const LIGHT_GREEN: Self = Self::from_rgb(0x90, 0xEE, 0x90);

    pub const DARK_BLUE: Self = Self::from_rgb(0, 0, 0x8B);
    pub const BLUE: Self = Self::from_rgb(0, 0, 255);
    pub const LIGHT_BLUE: Self = Self::from_rgb(0xAD, 0xD8, 0xE6);

    pub const GOLD: Self = Self::from_rgb(255, 215, 0);

    #[inline(always)]
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self([r, g, b, a])
    }

    #[inline(always)]
    pub const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self([r, g, b, 255])
    }

    #[inline(always)]
    pub const fn with_r(self, r: u8) -> Self {
        let Self([_, g, b, a]) = self;
        Self([r, g, b, a])
    }

    #[inline(always)]
    pub const fn with_g(self, g: u8) -> Self {
        let Self([r, _, b, a]) = self;
        Self([r, g, b, a])
    }

    #[inline(always)]
    pub const fn with_b(self, b: u8) -> Self {
        let Self([r, g, _, a]) = self;
        Self([r, g, b, a])
    }

    #[inline(always)]
    pub const fn with_a(self, a: u8) -> Self {
        let Self([r, g, b, _]) = self;
        Self([r, g, b, a])
    }

    #[inline(always)]
    pub const fn r(&self) -> &u8 {
        &self.0[0]
    }

    #[inline(always)]
    pub fn r_mut(&mut self) -> &mut u8 {
        &mut self.0[0]
    }

    #[inline(always)]
    pub const fn g(&self) -> &u8 {
        &self.0[1]
    }

    #[inline(always)]
    pub fn g_mut(&mut self) -> &mut u8 {
        &mut self.0[1]
    }

    #[inline(always)]
    pub const fn b(&self) -> &u8 {
        &self.0[2]
    }

    #[inline(always)]
    pub fn b_mut(&mut self) -> &mut u8 {
        &mut self.0[2]
    }

    #[inline(always)]
    pub const fn a(&self) -> &u8 {
        &self.0[3]
    }

    #[inline(always)]
    pub fn a_mut(&mut self) -> &mut u8 {
        &mut self.0[3]
    }
}

impl From<egui::Color32> for UColor32 {
    #[inline(always)]
    fn from(value: egui::Color32) -> Self {
        Self(value.to_srgba_unmultiplied())
    }
}

impl Into<egui::Color32> for UColor32 {
    #[inline(always)]
    fn into(self) -> egui::Color32 {
        let Self([r, g, b, a]) = self;
        egui::Color32::from_rgba_unmultiplied(r, g, b, a)
    }
}

impl From<[u8; 4]> for UColor32 {
    #[inline(always)]
    fn from(value: [u8; 4]) -> Self {
        Self(value)
    }
}

impl Into<[u8; 4]> for UColor32 {
    #[inline(always)]
    fn into(self) -> [u8; 4] {
        self.0
    }
}

impl From<(u8, u8, u8, u8)> for UColor32 {
    #[inline(always)]
    fn from(value: (u8, u8, u8, u8)) -> Self {
        let (r, g, b, a) = value;
        Self([r, g, b, a])
    }
}

impl Into<(u8, u8, u8, u8)> for UColor32 {
    #[inline(always)]
    fn into(self) -> (u8, u8, u8, u8) {
        let Self([r, g, b, a]) = self;
        (r, g, b, a)
    }
}
