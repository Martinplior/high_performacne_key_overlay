#![deny(unsafe_op_in_unsafe_fn)]

pub mod key;
pub mod key_bar;
pub mod key_drawer;
pub mod key_message;
pub mod key_overlay;
pub mod key_property;
pub mod kps_dashboard_app;
pub mod main_app;
pub mod message_dialog;
pub mod msg_hook;
pub mod setting;
pub mod setting_app;
pub mod u_color32;
pub mod utils;

const SETTING_FILE_NAME: &str = "setting.json";

pub fn get_current_dir() -> std::path::PathBuf {
    std::env::current_dir().unwrap_or_else(|err| {
        message_dialog::error(format!("未知错误：{}", err.to_string())).show();
        panic!()
    })
}

pub fn key_overlay_setting_path() -> std::path::PathBuf {
    get_current_dir().join(SETTING_FILE_NAME)
}

pub fn graceful_run<R>(
    f: impl FnOnce() -> R + std::panic::UnwindSafe,
) -> Result<R, Box<dyn std::any::Any + Send>> {
    std::panic::catch_unwind(f).map_err(|err| {
        let message = if let Some(err) = err.downcast_ref::<String>() {
            err.clone()
        } else {
            format!("{:?}, type_id = {:?}", err, err.type_id())
        };
        #[cfg(debug_assertions)]
        dbg!(&message);
        message_dialog::error(message).show();
        err
    })
}

#[cfg(test)]
mod tests {

    use egui::{Color32, FontDefinitions};

    use crate::{setting::Setting, u_color32::UColor32};

    #[test]
    fn query_fonts() {
        let sys_fonts = font_kit::source::SystemSource::new();
        let families = sys_fonts.all_families().unwrap();
        families.iter().enumerate().for_each(|(index, family)| {
            let family_handle = sys_fonts.select_family_by_name(family).unwrap();
            let fonts = family_handle.fonts();
            fonts.iter().for_each(|handle| {
                let font = handle.load().unwrap();
                let font_index = match handle {
                    font_kit::handle::Handle::Path { font_index, .. } => font_index,
                    _ => unreachable!(),
                };
                println!(
                    "{}: {}, {}, font_index: {}, {:?}",
                    index,
                    family,
                    font.full_name(),
                    font_index,
                    font.properties()
                );
            });
        });
    }

    #[test]
    fn serialize() {
        let setting = Setting::default();
        let setting_json = serde_json::to_string_pretty(&setting).unwrap();
        let _setting_1 = serde_json::from_str::<Setting>(&setting_json).unwrap();
        println!("{}", setting_json);
    }

    #[test]
    fn builtin_font_names() {
        println!("{:?}", FontDefinitions::builtin_font_names());
    }

    #[test]
    fn tmp() {
        let ucolor = UColor32::WHITE.with_a(128);
        let color: Color32 = ucolor.into();
        println!("{:?}\n{:?}", ucolor, color);
    }
}
