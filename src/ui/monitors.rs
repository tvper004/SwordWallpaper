use crate::app::{AuraApp, Message};
use crate::scanner::IMAGE_EXTENSIONS;
use cosmic::iced::alignment::Horizontal;
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;

impl AuraApp {
    pub(crate) fn view_monitors(&self) -> Element<'_, Message> {
        let mut col = widget::column::with_capacity(self.outputs.len() + 4)
            .spacing(18)
            .width(Length::Fill);

        col = col.push(widget::text::title2(self.language.monitors_title()));
        col = col.push(widget::text::body(self.language.monitors_desc()));

        // Target Selector when multiple outputs are present
        if self.outputs.len() > 1 {
            let is_all = self.selected_output == "*";
            let mut selector_row = widget::row::with_capacity(self.outputs.len() + 2)
                .spacing(10)
                .align_y(Alignment::Center);

            selector_row = selector_row.push(
                widget::text::body(self.language.library_target_output_label())
            );

            let all_btn = if is_all {
                widget::button::suggested(format!("✓ {} (Replicar)", self.language.monitors_all_displays()))
                    .leading_icon(widget::icon::from_name("emblem-ok-symbolic"))
            } else {
                widget::button::standard(format!("{} (Replicar)", self.language.monitors_all_displays()))
                    .leading_icon(widget::icon::from_name("video-display-symbolic"))
            }
            .on_press(Message::SelectOutput("*".into()));

            selector_row = selector_row.push(all_btn);

            for monitor in &self.outputs {
                let is_sel = self.selected_output == monitor.name;
                let btn = if is_sel {
                    widget::button::suggested(&monitor.name)
                        .leading_icon(widget::icon::from_name("emblem-ok-symbolic"))
                } else {
                    widget::button::standard(&monitor.name)
                        .leading_icon(widget::icon::from_name("video-display-symbolic"))
                }
                .on_press(Message::SelectOutput(monitor.name.clone()));
                selector_row = selector_row.push(btn);
            }

            col = col.push(selector_row);
        }

        // Horizontal Canvas of Virtual Monitors
        let mut monitors_canvas = widget::row::with_capacity(self.outputs.len() + 2)
            .spacing(24)
            .align_y(Alignment::End);

        // If multiple monitors, add a "All Displays (Replicate)" card
        if self.outputs.len() > 1 {
            let active_wall = self.config.wallpapers.get("*")
                .or_else(|| self.config.current.as_ref())
                .or_else(|| self.config.wallpapers.values().next());
            let active_scaling = self.config.scaling.get("*")
                .map(|s| s.as_str())
                .unwrap_or("fit");

            let screen_w = 260.0f32;
            let is_target = self.selected_output == "*";

            let screen_display: Element<'_, Message> = if let Some(wall_path) = active_wall {
                let p = std::path::Path::new(wall_path);
                let is_image = p.extension()
                    .and_then(|e| e.to_str())
                    .map(|ext| IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
                    .unwrap_or(false);

                let display_thumb = if is_image && p.exists() {
                    Some(p.to_path_buf())
                } else {
                    let thumb = crate::scanner::thumbs::thumb_path_for_video(p);
                    if thumb.exists() {
                        Some(thumb)
                    } else if p.exists() && is_image {
                        Some(p.to_path_buf())
                    } else {
                        None
                    }
                };

                if let Some(thumb_path) = display_thumb {
                    Element::from(
                        widget::button::image(thumb_path.to_string_lossy().to_string())
                            .width(screen_w)
                            .selected(is_target)
                            .on_press(Message::SelectOutput("*".into()))
                    )
                } else {
                    Element::from(
                        widget::button::standard(self.language.monitors_all_displays())
                            .width(screen_w)
                            .on_press(Message::SelectOutput("*".into()))
                    )
                }
            } else {
                Element::from(
                    widget::button::standard(self.language.monitors_all_displays())
                        .width(screen_w)
                        .on_press(Message::SelectOutput("*".into()))
                )
            };

            let target_btn = if is_target {
                widget::button::suggested(self.language.monitors_target_active())
                    .leading_icon(widget::icon::from_name("emblem-ok-symbolic"))
            } else {
                widget::button::standard(self.language.monitors_all_displays())
                    .leading_icon(widget::icon::from_name("video-display-symbolic"))
            }
            .on_press(Message::SelectOutput("*".into()));

            let all_card = widget::column::with_capacity(7)
                .spacing(10)
                .padding(14)
                .align_x(Horizontal::Center)
                .push(widget::text::title3(format!("{} (Replicar) • {}", self.language.monitors_all_displays(), self.outputs.len())))
                .push(screen_display)
                .push(target_btn)
                .push(
                    widget::row::with_capacity(4)
                        .spacing(8)
                        .align_y(Alignment::Center)
                        .push(
                            if active_scaling == "fit" {
                                widget::button::suggested(self.language.scaling_fit())
                            } else {
                                widget::button::standard(self.language.scaling_fit())
                            }.on_press(Message::SelectScaling { output: "*".into(), scaling: "fit".into() })
                        )
                        .push(
                            if active_scaling == "fill" {
                                widget::button::suggested(self.language.scaling_fill())
                            } else {
                                widget::button::standard(self.language.scaling_fill())
                            }.on_press(Message::SelectScaling { output: "*".into(), scaling: "fill".into() })
                        )
                        .push(
                            if active_scaling == "stretch" {
                                widget::button::suggested(self.language.scaling_stretch())
                            } else {
                                widget::button::standard(self.language.scaling_stretch())
                            }.on_press(Message::SelectScaling { output: "*".into(), scaling: "stretch".into() })
                        )
                        .push(
                            widget::button::destructive(self.language.monitors_stop())
                                .leading_icon(widget::icon::from_name("process-stop-symbolic"))
                                .on_press(Message::StopWallpaper(Some("*".into())))
                        )
                );

            monitors_canvas = monitors_canvas.push(widget::container(all_card));
        }

        for monitor in &self.outputs {
            let active_wall = self.config.wallpapers.get(&monitor.name)
                .or_else(|| self.config.wallpapers.get("*"))
                .or_else(|| self.config.current.as_ref());
            let active_scaling = self.config.scaling.get(&monitor.name)
                .or_else(|| self.config.scaling.get("*"))
                .map(|s| s.as_str())
                .unwrap_or("fit");

            let screen_w = 260.0f32;
            let is_target = self.selected_output == monitor.name;
            let is_all = self.selected_output == "*";

            // Screen frame representation with image & video thumbnail support
            let screen_display: Element<'_, Message> = if let Some(wall_path) = active_wall {
                let p = std::path::Path::new(wall_path);
                let is_image = p.extension()
                    .and_then(|e| e.to_str())
                    .map(|ext| IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
                    .unwrap_or(false);

                let display_thumb = if is_image && p.exists() {
                    Some(p.to_path_buf())
                } else {
                    let thumb = crate::scanner::thumbs::thumb_path_for_video(p);
                    if thumb.exists() {
                        Some(thumb)
                    } else if p.exists() && is_image {
                        Some(p.to_path_buf())
                    } else {
                        None
                    }
                };

                if let Some(thumb_path) = display_thumb {
                    Element::from(
                        widget::button::image(thumb_path.to_string_lossy().to_string())
                            .width(screen_w)
                            .selected(is_target || is_all)
                            .on_press(if is_target {
                                Message::SelectOutput("*".into())
                            } else {
                                Message::SelectOutput(monitor.name.clone())
                            })
                    )
                } else {
                    Element::from(
                        widget::button::standard(self.language.monitors_screen_name(&monitor.name))
                            .width(screen_w)
                            .on_press(if is_target {
                                Message::SelectOutput("*".into())
                            } else {
                                Message::SelectOutput(monitor.name.clone())
                            })
                    )
                }
            } else {
                Element::from(
                    widget::button::standard(self.language.monitors_screen_idle(&monitor.name))
                        .width(screen_w)
                        .on_press(if is_target {
                            Message::SelectOutput("*".into())
                        } else {
                            Message::SelectOutput(monitor.name.clone())
                        })
                )
            };

            let target_btn = if is_target {
                widget::button::suggested(self.language.monitors_target_active())
                    .leading_icon(widget::icon::from_name("emblem-ok-symbolic"))
                    .on_press(Message::SelectOutput("*".into()))
            } else if is_all {
                widget::button::suggested(format!("✓ {}", self.language.monitors_target_active()))
                    .leading_icon(widget::icon::from_name("emblem-ok-symbolic"))
                    .on_press(Message::SelectOutput(monitor.name.clone()))
            } else {
                widget::button::standard(self.language.monitors_apply_to_this())
                    .leading_icon(widget::icon::from_name("video-display-symbolic"))
                    .on_press(Message::SelectOutput(monitor.name.clone()))
            };

            let mut card_actions = widget::row::with_capacity(2)
                .spacing(8)
                .align_y(Alignment::Center)
                .push(target_btn);

            if self.outputs.len() > 1 && !is_all {
                card_actions = card_actions.push(
                    widget::button::standard(self.language.monitors_all_displays())
                        .leading_icon(widget::icon::from_name("video-display-symbolic"))
                        .on_press(Message::SelectOutput("*".into()))
                );
            }

            let monitor_card = widget::column::with_capacity(7)
                .spacing(10)
                .padding(14)
                .align_x(Horizontal::Center)
                .push(widget::text::title3(format!("{} • {}", monitor.name, monitor.resolution)))
                .push(screen_display)
                .push(card_actions)
                .push(
                    widget::row::with_capacity(4)
                        .spacing(8)
                        .align_y(Alignment::Center)
                        .push(
                            if active_scaling == "fit" {
                                widget::button::suggested(self.language.scaling_fit())
                            } else {
                                widget::button::standard(self.language.scaling_fit())
                            }.on_press(Message::SelectScaling { output: monitor.name.clone(), scaling: "fit".into() })
                        )
                        .push(
                            if active_scaling == "fill" {
                                widget::button::suggested(self.language.scaling_fill())
                            } else {
                                widget::button::standard(self.language.scaling_fill())
                            }.on_press(Message::SelectScaling { output: monitor.name.clone(), scaling: "fill".into() })
                        )
                        .push(
                            if active_scaling == "stretch" {
                                widget::button::suggested(self.language.scaling_stretch())
                            } else {
                                widget::button::standard(self.language.scaling_stretch())
                            }.on_press(Message::SelectScaling { output: monitor.name.clone(), scaling: "stretch".into() })
                        )
                        .push(
                            widget::button::destructive(self.language.monitors_stop())
                                .leading_icon(widget::icon::from_name("process-stop-symbolic"))
                                .on_press(Message::StopWallpaper(Some(monitor.name.clone())))
                        )
                );

            monitors_canvas = monitors_canvas.push(widget::container(monitor_card));
        }

        col = col.push(widget::scrollable(monitors_canvas).scrollbar_width(0.0).scroller_width(0.0));

        Element::from(col)
    }
}
