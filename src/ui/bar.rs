use crate::app::{AuraApp, Message};
use cosmic::iced::alignment::Horizontal;
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;

impl AuraApp {
    pub(crate) fn view_lockscreen_conversion_bar(&self) -> Option<Element<'_, Message>> {
        let conversion = self.lockscreen_conversion.as_ref()?;
        let name = conversion.job.path.file_stem()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_else(|| conversion.job.path.to_string_lossy().to_string());
        let progress = widget::progress_bar::determinate_linear(conversion.percent as f32 / 100.0)
            .width(Length::Fill);
        let mut content = widget::column::with_capacity(3)
            .spacing(5)
            .width(Length::Fill)
            .push(widget::text::caption(self.language.lockscreen_conversion_progress(
                &name,
                &conversion.job.output,
                conversion.percent,
            )))
            .push(progress);

        if let Some(next) = self.pending_lockscreen_conversions.front() {
            let next_name = next.path.file_stem()
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or_else(|| next.path.to_string_lossy().to_string());
            content = content.push(widget::text::caption(self.language.lockscreen_conversion_next(
                &next_name,
                &next.output,
                self.pending_lockscreen_conversions.len(),
            )));
        }

        Some(Element::from(
            widget::container(content)
                .padding([7, 12])
                .width(Length::Fill)
                .class(cosmic::theme::Container::Card),
        ))
    }

    pub(crate) fn view_now_playing_bar(&self) -> Element<'_, Message> {
        let has_wallpapers = !self.config.wallpapers.is_empty();

        let (wall_title, out_label) = if let Some(curr) = &self.config.current {
            let p = std::path::Path::new(curr);
            let name = p
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| self.language.bar_active_title_default().into());
            let out_display = if self.config.wallpapers.contains_key("*")
                || self.selected_output == "*"
                || (self.outputs.len() > 1 && self.outputs.iter().all(|m| self.config.wallpapers.contains_key(&m.name)))
            {
                self.language.monitors_all_displays().to_string()
            } else {
                let list = self.config.wallpapers.keys().cloned().collect::<Vec<_>>().join(", ");
                if list.is_empty() { self.selected_output.clone() } else { list }
            };
            let base_label = self.language.bar_display_label(&out_display);
            let rot_badge = if self.config.rotation {
                if self.config.rotation_only_favorites {
                    format!(" • ♥ {}m", self.config.interval)
                } else {
                    format!(" • ⟳ {}m", self.config.interval)
                }
            } else {
                String::new()
            };
            (name, format!("{}{}", base_label, rot_badge))
        } else {
            (self.language.bar_idle_title().into(), self.language.bar_idle_desc().into())
        };

        let is_paused = self.is_paused;
        let is_muted = self.config.mute;
        let volume = self.config.volume;
        let dark = cosmic::theme::is_dark();
        #[cfg(not(target_os = "macos"))]
        let hwdec = &self.config.hwdec;
        let lang = self.language;

        // Previous & Next buttons with tooltips
        let prev_btn = widget::button::icon(widget::icon::from_name("media-skip-backward-symbolic"))
            .tooltip(lang.bar_prev())
            .on_press_maybe(if has_wallpapers { Some(Message::PrevWallpaper) } else { None });

        let next_btn = widget::button::icon(widget::icon::from_name("media-skip-forward-symbolic"))
            .tooltip(lang.bar_next())
            .on_press_maybe(if has_wallpapers { Some(Message::NextWallpaper) } else { None });

        // Play / Pause toggle with tooltip and suggested highlight when paused
        let play_pause_btn = if is_paused {
            widget::button::icon(widget::icon::from_name("media-playback-start-symbolic"))
                .class(cosmic::theme::Button::Suggested)
                .tooltip(lang.bar_resume())
                .on_press_maybe(if has_wallpapers { Some(Message::TogglePause) } else { None })
        } else {
            widget::button::icon(widget::icon::from_name("media-playback-pause-symbolic"))
                .tooltip(lang.bar_pause())
                .on_press_maybe(if has_wallpapers { Some(Message::TogglePause) } else { None })
        };

        // Mute / Unmute toggle with tooltip and suggested highlight when sound is active
        let mute_btn = if is_muted {
            widget::button::icon(widget::icon::from_name("audio-volume-muted-symbolic"))
                .tooltip(lang.bar_muted())
                .on_press(Message::ToggleMute(false))
        } else {
            widget::button::icon(widget::icon::from_name("audio-volume-high-symbolic"))
                .class(cosmic::theme::Button::Suggested)
                .tooltip(format!("{}: {}%", lang.bar_audio_active(), volume))
                .on_press(Message::ToggleMute(true))
        };

        // Stop button with tooltip and destructive styling
        let stop_btn = widget::button::icon(widget::icon::from_name("process-stop-symbolic"))
            .class(cosmic::theme::Button::Destructive)
            .tooltip(lang.bar_stop())
            .on_press_maybe(if has_wallpapers { Some(Message::StopWallpaper(None)) } else { None });

        let mut center_controls = widget::row::with_capacity(7)
            .spacing(8)
            .align_y(Alignment::Center)
            .push(prev_btn)
            .push(play_pause_btn)
            .push(next_btn)
            .push(mute_btn);

        if !is_muted {
            let vol_slider = widget::slider(0..=100, volume, Message::ChangeVolume)
                .width(Length::Fixed(75.0));
            let vol_text = widget::text::caption(format!("{}%", volume));
            center_controls = center_controls.push(vol_slider).push(vol_text);
        }

        center_controls = center_controls.push(stop_btn);

        let left_col = widget::column::with_capacity(2)
            .spacing(2)
            .width(Length::Fill)
            .push(widget::text::body(wall_title).size(14))
            .push(widget::text::caption(out_label));

        #[cfg(target_os = "macos")]
        let right_col = widget::column::with_capacity(1)
            .spacing(2)
            .align_x(Horizontal::Right)
            .push(widget::text::caption(lang.bar_wayland_tag()));

        #[cfg(not(target_os = "macos"))]
        let right_col = widget::column::with_capacity(2)
            .spacing(2)
            .align_x(Horizontal::Right)
            .push(widget::text::caption(lang.bar_gpu_accel(hwdec)))
            .push(widget::text::caption(lang.bar_wayland_tag()));

        let bar_content = widget::row::with_capacity(3)
            .spacing(16)
            .align_y(Alignment::Center)
            .width(Length::Fill)
            .push(left_col)
            .push(center_controls)
            .push(right_col);

        Element::from(
            widget::container(bar_content)
                .padding([8, 14])
                .width(Length::Fill)
                .class(crate::ui::glass_panel(dark, if dark { 0.82 } else { 0.92 }, 24.0))
        )
    }
}
