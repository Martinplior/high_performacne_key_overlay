use egui::TexturesDelta;

use crate::{epi, App};

use super::{now_sec, text_agent::TextAgent, web_painter::WebPainter, NeedRepaint};

pub struct AppRunner {
    #[allow(dead_code)]
    pub(crate) web_options: crate::WebOptions,
    pub(crate) frame: epi::Frame,
    egui_ctx: egui::Context,
    painter: super::ActiveWebPainter,
    pub(crate) input: super::WebInput,
    app: Box<dyn epi::App>,
    pub(crate) needs_repaint: std::sync::Arc<NeedRepaint>,
    last_save_time: f64,
    pub(crate) text_agent: TextAgent,

    // Output for the last run:
    textures_delta: TexturesDelta,
    clipped_primitives: Option<Vec<egui::ClippedPrimitive>>,
}

impl Drop for AppRunner {
    fn drop(&mut self) {
        log::debug!("AppRunner has fully dropped");
    }
}

impl AppRunner {
    /// # Errors
    /// Failure to initialize WebGL renderer, or failure to create app.
    pub async fn new(
        canvas: web_sys::HtmlCanvasElement,
        web_options: crate::WebOptions,
        app_creator: epi::AppCreator<'static>,
        text_agent: TextAgent,
    ) -> Result<Self, String> {
        let painter = super::ActiveWebPainter::new(canvas, &web_options).await?;

        let info = epi::IntegrationInfo {
            web_info: epi::WebInfo {
                user_agent: super::user_agent().unwrap_or_default(),
                location: super::web_location(),
            },
            cpu_usage: None,
        };
        let storage = LocalStorage::default();

        let egui_ctx = egui::Context::default();
        egui_ctx.set_os(egui::os::OperatingSystem::from_user_agent(
            &super::user_agent().unwrap_or_default(),
        ));
        super::storage::load_memory(&egui_ctx);

        egui_ctx.options_mut(|o| {
            // On web by default egui follows the zoom factor of the browser,
            // and lets the browser handle the zoom shortscuts.
            // A user can still zoom egui separately by calling [`egui::Context::set_zoom_factor`].
            o.zoom_with_keyboard = false;
            o.zoom_factor = 1.0;
        });

        let cc = epi::CreationContext {
            egui_ctx: egui_ctx.clone(),
            integration_info: info.clone(),
            storage: Some(&storage),

            #[cfg(feature = "glow")]
            gl: Some(painter.gl().clone()),

            #[cfg(feature = "glow")]
            get_proc_address: None,

            #[cfg(all(feature = "wgpu", not(feature = "glow")))]
            wgpu_render_state: painter.render_state(),
            #[cfg(all(feature = "wgpu", feature = "glow"))]
            wgpu_render_state: None,
        };
        let app = app_creator(&cc).map_err(|err| err.to_string())?;

        let frame = epi::Frame {
            info,
            storage: Some(Box::new(storage)),

            #[cfg(feature = "glow")]
            gl: Some(painter.gl().clone()),

            #[cfg(all(feature = "wgpu", not(feature = "glow")))]
            wgpu_render_state: painter.render_state(),
            #[cfg(all(feature = "wgpu", feature = "glow"))]
            wgpu_render_state: None,
        };

        let needs_repaint: std::sync::Arc<NeedRepaint> = Default::default();
        {
            let needs_repaint = needs_repaint.clone();
            egui_ctx.set_request_repaint_callback(move |info| {
                needs_repaint.repaint_after(info.delay.as_secs_f64());
            });
        }

        let mut runner = Self {
            web_options,
            frame,
            egui_ctx,
            painter,
            input: Default::default(),
            app,
            needs_repaint,
            last_save_time: now_sec(),
            text_agent,
            textures_delta: Default::default(),
            clipped_primitives: None,
        };

        runner.input.raw.max_texture_side = Some(runner.painter.max_texture_side());
        runner
            .input
            .raw
            .viewports
            .entry(egui::ViewportId::ROOT)
            .or_default()
            .native_pixels_per_point = Some(super::native_pixels_per_point());
        runner.input.raw.system_theme = super::system_theme();

        Ok(runner)
    }

    pub fn egui_ctx(&self) -> &egui::Context {
        &self.egui_ctx
    }

    /// Get mutable access to the concrete [`App`] we enclose.
    ///
    /// This will panic if your app does not implement [`App::as_any_mut`].
    pub fn app_mut<ConcreteApp: 'static + App>(&mut self) -> &mut ConcreteApp {
        self.app
            .as_any_mut()
            .expect("Your app must implement `as_any_mut`, but it doesn't")
            .downcast_mut::<ConcreteApp>()
            .expect("app_mut got the wrong type of App")
    }

    pub fn auto_save_if_needed(&mut self) {
        let time_since_last_save = now_sec() - self.last_save_time;
        if time_since_last_save > self.app.auto_save_interval().as_secs_f64() {
            self.save();
        }
    }

    pub fn save(&mut self) {
        if self.app.persist_egui_memory() {
            super::storage::save_memory(&self.egui_ctx);
        }
        if let Some(storage) = self.frame.storage_mut() {
            self.app.save(storage);
        }
        self.last_save_time = now_sec();
    }

    pub fn canvas(&self) -> &web_sys::HtmlCanvasElement {
        self.painter.canvas()
    }

    pub fn destroy(mut self) {
        log::debug!("Destroying AppRunner");
        self.painter.destroy();
    }

    pub fn has_outstanding_paint_data(&self) -> bool {
        self.clipped_primitives.is_some()
    }

    /// Does the eframe app have focus?
    ///
    /// Technically: does either the canvas or the [`TextAgent`] have focus?
    pub fn has_focus(&self) -> bool {
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        if document.hidden() {
            return false;
        }

        super::has_focus(self.canvas()) || self.text_agent.has_focus()
    }

    pub fn update_focus(&mut self) {
        let has_focus = self.has_focus();
        if self.input.raw.focused != has_focus {
            log::trace!("{} Focus changed to {has_focus}", self.canvas().id());
            self.input.set_focus(has_focus);

            if !has_focus {
                // We lost focus - good idea to save
                self.save();
            }
            self.egui_ctx().request_repaint();
        }
    }

    /// Runs the logic, but doesn't paint the result.
    ///
    /// The result can be painted later with a call to [`Self::run_and_paint`] or [`Self::paint`].
    pub fn logic(&mut self) {
        // We sometimes miss blur/focus events due to the text agent, so let's just poll each frame:
        self.update_focus();

        let canvas_size = super::canvas_size_in_points(self.canvas(), self.egui_ctx());
        let mut raw_input = self.input.new_frame(canvas_size);

        self.app.raw_input_hook(&self.egui_ctx, &mut raw_input);

        let full_output = self.egui_ctx.run(raw_input, |egui_ctx| {
            self.app.update(egui_ctx, &mut self.frame);
        });
        let egui::FullOutput {
            platform_output,
            textures_delta,
            shapes,
            pixels_per_point,
            viewport_output,
        } = full_output;

        if viewport_output.len() > 1 {
            log::warn!("Multiple viewports not yet supported on the web");
        }
        for viewport_output in viewport_output.values() {
            for command in &viewport_output.commands {
                // TODO(emilk): handle some of the commands
                log::warn!(
                    "Unhandled egui viewport command: {command:?} - not implemented in web backend"
                );
            }
        }

        self.handle_platform_output(platform_output);
        self.textures_delta.append(textures_delta);
        self.clipped_primitives = Some(self.egui_ctx.tessellate(shapes, pixels_per_point));
    }

    /// Paint the results of the last call to [`Self::logic`].
    pub fn paint(&mut self) {
        let textures_delta = std::mem::take(&mut self.textures_delta);
        let clipped_primitives = std::mem::take(&mut self.clipped_primitives);

        if let Some(clipped_primitives) = clipped_primitives {
            if let Err(err) = self.painter.paint_and_update_textures(
                self.app.clear_color(&self.egui_ctx.style().visuals),
                &clipped_primitives,
                self.egui_ctx.pixels_per_point(),
                &textures_delta,
            ) {
                log::error!("Failed to paint: {}", super::string_from_js_value(&err));
            }
        }
    }

    pub fn report_frame_time(&mut self, cpu_usage_seconds: f32) {
        self.frame.info.cpu_usage = Some(cpu_usage_seconds);
    }

    fn handle_platform_output(&mut self, platform_output: egui::PlatformOutput) {
        #[cfg(feature = "web_screen_reader")]
        if self.egui_ctx.options(|o| o.screen_reader) {
            super::screen_reader::speak(&platform_output.events_description());
        }

        let egui::PlatformOutput {
            cursor_icon,
            open_url,
            copied_text,
            events: _,                    // already handled
            mutable_text_under_cursor: _, // TODO(#4569): https://github.com/emilk/egui/issues/4569
            ime,
            #[cfg(feature = "accesskit")]
                accesskit_update: _, // not currently implemented
            num_completed_passes: _,    // handled by `Context::run`
            request_discard_reasons: _, // handled by `Context::run`
        } = platform_output;

        super::set_cursor_icon(cursor_icon);
        if let Some(open) = open_url {
            super::open_url(&open.url, open.new_tab);
        }

        if !copied_text.is_empty() {
            super::set_clipboard_text(&copied_text);
        }

        if self.has_focus() {
            // The eframe app has focus.
            if ime.is_some() {
                // We are editing text: give the focus to the text agent.
                self.text_agent.focus();
            } else {
                // We are not editing text - give the focus to the canvas.
                self.text_agent.blur();
                self.canvas().focus().ok();
            }
        }

        if let Err(err) = self
            .text_agent
            .move_to(ime, self.canvas(), self.egui_ctx.zoom_factor())
        {
            log::error!(
                "failed to update text agent position: {}",
                super::string_from_js_value(&err)
            );
        }
    }
}

// ----------------------------------------------------------------------------

#[derive(Default)]
struct LocalStorage {}

impl epi::Storage for LocalStorage {
    fn get_string(&self, key: &str) -> Option<String> {
        super::storage::local_storage_get(key)
    }

    fn set_string(&mut self, key: &str, value: String) {
        super::storage::local_storage_set(key, &value);
    }

    fn flush(&mut self) {}
}