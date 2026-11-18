use crate::app::{AuraApp, LibraryFilter, LibrarySort, Message};
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;

impl AuraApp {
    pub(crate) fn view_library(&self) -> Element<'_, Message> {
        match self.config.switcher_style {
            crate::config::SwitcherStyle::Classic => self.view_classic_library(),
            crate::config::SwitcherStyle::Cinematic => self.view_cinematic_library(),
            crate::config::SwitcherStyle::Honeycomb => self.view_honeycomb_library(),
        }
    }

    pub(crate) fn view_classic_library(&self) -> Element<'_, Message> {
        let search_input = widget::text_input::search_input(self.language.library_search_placeholder(), &self.search_query)
            .on_input(Message::SearchChanged)
            .width(Length::Fill);

        let (sort_icon, sort_label) = match self.library_sort {
            LibrarySort::Newest => ("view-sort-descending-symbolic", self.language.library_sort_newest()),
            LibrarySort::Oldest => ("view-sort-ascending-symbolic", self.language.library_sort_oldest()),
        };

        let sort_btn = widget::button::standard(sort_label)
            .leading_icon(widget::icon::from_name(sort_icon))
            .on_press(Message::ToggleLibrarySort);

        let search_row = widget::row::with_capacity(2)
            .spacing(10)
            .align_y(Alignment::Center)
            .push(search_input)
            .push(sort_btn);

        // Quick category filter counts (precalculated in O(1))
        let (count_all, count_live, count_static, count_downloaded, count_favorites) = self.category_counts;

        let all_btn = if self.library_filter == LibraryFilter::All {
            widget::button::suggested(format!("{} ({})", self.language.library_filter_all(), count_all))
        } else {
            widget::button::standard(format!("{} ({})", self.language.library_filter_all(), count_all))
        }
        .leading_icon(widget::icon::from_name("view-grid-symbolic"))
        .on_press(Message::SelectLibraryFilter(LibraryFilter::All));

        let live_btn = if self.library_filter == LibraryFilter::Live {
            widget::button::suggested(format!("{} ({})", self.language.library_filter_live(), count_live))
        } else {
            widget::button::standard(format!("{} ({})", self.language.library_filter_live(), count_live))
        }
        .leading_icon(widget::icon::from_name("video-x-generic-symbolic"))
        .on_press(Message::SelectLibraryFilter(LibraryFilter::Live));

        let static_btn = if self.library_filter == LibraryFilter::Static {
            widget::button::suggested(format!("{} ({})", self.language.library_filter_static(), count_static))
        } else {
            widget::button::standard(format!("{} ({})", self.language.library_filter_static(), count_static))
        }
        .leading_icon(widget::icon::from_name("image-x-generic-symbolic"))
        .on_press(Message::SelectLibraryFilter(LibraryFilter::Static));

        let downloaded_btn = if self.library_filter == LibraryFilter::Downloaded {
            widget::button::suggested(format!("{} ({})", self.language.library_filter_downloaded(), count_downloaded))
        } else {
            widget::button::standard(format!("{} ({})", self.language.library_filter_downloaded(), count_downloaded))
        }
        .leading_icon(widget::icon::from_name("folder-download-symbolic"))
        .on_press(Message::SelectLibraryFilter(LibraryFilter::Downloaded));

        let favorites_btn = if self.library_filter == LibraryFilter::Favorites {
            widget::button::suggested(format!("{} ({})", self.language.library_filter_favorites(), count_favorites))
        } else {
            widget::button::standard(format!("{} ({})", self.language.library_filter_favorites(), count_favorites))
        }
        .leading_icon(widget::icon::from_name("emblem-favorite-symbolic"))
        .on_press(Message::SelectLibraryFilter(LibraryFilter::Favorites));

        let open_folder_btn = widget::button::standard(self.language.open_in_file_manager())
            .leading_icon(widget::icon::from_name("folder-open-symbolic"))
            .on_press(Message::OpenWallpapersFolder);

        let filter_bar = widget::row::with_capacity(6)
            .spacing(8)
            .align_y(Alignment::Center)
            .push(all_btn)
            .push(live_btn)
            .push(static_btn)
            .push(downloaded_btn)
            .push(favorites_btn)
            .push(open_folder_btn);

        let mut top_section = widget::column::with_capacity(4)
            .spacing(10)
            .push(search_row)
            .push(widget::scrollable::horizontal(filter_bar).scrollbar_width(0.0).scroller_width(0.0));

        // Favorites quick-rotation banner when browsing Favorites filter tab
        if self.library_filter == LibraryFilter::Favorites && count_favorites > 0 {
            let is_fav_rotation = self.config.rotation && self.config.rotation_only_favorites;
            let heart_icon = widget::icon::from_name("emblem-favorite-symbolic").size(24);

            let banner_info = if is_fav_rotation {
                let is_random = self.config.order == "random";
                widget::column::with_capacity(2)
                    .spacing(2)
                    .push(widget::text::title3(self.language.library_favs_rotation_title()))
                    .push(widget::text::caption(self.language.library_favs_banner_desc_active(count_favorites, self.config.interval, is_random)))
            } else {
                widget::column::with_capacity(2)
                    .spacing(2)
                    .push(widget::text::title3(self.language.library_favs_rotation_title()))
                    .push(widget::text::caption(self.language.library_favs_banner_desc_idle(count_favorites)))
            };

            let mut banner_actions = widget::row::with_capacity(2)
                .spacing(8)
                .align_y(Alignment::Center);

            if is_fav_rotation {
                banner_actions = banner_actions.push(
                    widget::button::standard(self.language.library_favs_stop_rotation())
                        .leading_icon(widget::icon::from_name("media-playback-pause-symbolic"))
                        .on_press(Message::ToggleRotation(false))
                );
            } else {
                banner_actions = banner_actions.push(
                    widget::button::suggested(self.language.library_favs_start_rotation())
                        .leading_icon(widget::icon::from_name("media-playlist-repeat-symbolic"))
                        .on_press(Message::StartFavoritesRotation)
                );
            }

            banner_actions = banner_actions.push(
                widget::button::icon(widget::icon::from_name("preferences-system-symbolic"))
                    .tooltip(self.language.library_favs_rotation_settings())
                    .on_press(Message::NavigateToPage(crate::app::Page::Settings))
            );

            let banner_row = widget::row::with_capacity(3)
                .spacing(12)
                .align_y(Alignment::Center)
                .width(Length::Fill)
                .push(heart_icon)
                .push(widget::container(banner_info).width(Length::Fill))
                .push(banner_actions);

            let banner_box = widget::container(banner_row)
                .padding(12)
                .width(Length::Fill);

            top_section = top_section.push(banner_box);
        }

        // Target display selector chips
        if self.outputs.len() > 1 {
            let mut out_row = widget::row::with_capacity(self.outputs.len() + 2)
                .spacing(8)
                .align_y(Alignment::Center);

            out_row = out_row.push(widget::text::body(self.language.library_target_output_label()));

            let all_sel = self.selected_output == "*";
            let all_chip = if all_sel {
                widget::button::suggested(self.language.monitors_all_displays())
            } else {
                widget::button::standard(self.language.monitors_all_displays())
            }
            .leading_icon(widget::icon::from_name("video-display-symbolic"))
            .on_press(Message::SelectOutput("*".into()));
            out_row = out_row.push(all_chip);

            for out in &self.outputs {
                let is_sel = self.selected_output == out.name;
                let chip = if is_sel {
                    widget::button::suggested(&out.name)
                } else {
                    widget::button::standard(&out.name)
                }
                .leading_icon(widget::icon::from_name("video-display-symbolic"))
                .on_press(Message::SelectOutput(out.name.clone()));
                out_row = out_row.push(chip);
            }

            top_section = top_section.push(widget::scrollable::horizontal(out_row).scrollbar_width(0.0).scroller_width(0.0));
        }

        if self.videos.is_empty() {
            let empty_msg = widget::column::with_capacity(4)
                .spacing(14)
                .align_x(Horizontal::Center)
                .push(widget::text::title2(self.language.library_empty_title()))
                .push(widget::text::body(self.language.library_empty_desc()))
                .push(
                    widget::row::with_capacity(2)
                        .spacing(12)
                        .push(
                            widget::button::suggested(self.language.library_btn_add_video())
                                .leading_icon(widget::icon::from_name("list-add-symbolic"))
                                .on_press(Message::PickVideoFile)
                        )
                        .push(
                            widget::button::standard(self.language.library_btn_add_folder())
                                .leading_icon(widget::icon::from_name("folder-symbolic"))
                                .on_press(Message::PickFolder)
                        )
                );

            return Element::from(
                widget::container(empty_msg)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Center)
            );
        }

        let search_trimmed = self.search_query.trim();
        let query_lower = if search_trimmed.is_empty() {
            None
        } else {
            Some(search_trimmed.to_lowercase())
        };

        let filtered_videos: Vec<_> = self.videos.iter().filter(|v| {
            let match_type = match self.library_filter {
                LibraryFilter::All => true,
                LibraryFilter::Live => v.is_video,
                LibraryFilter::Static => !v.is_video,
                LibraryFilter::Downloaded => v.is_downloaded,
                LibraryFilter::Favorites => self.favorites_set.contains(v.path.to_string_lossy().as_ref()),
            };

            if !match_type {
                return false;
            }

            if let Some(ref q) = query_lower {
                v.name_lower.contains(q)
            } else {
                true
            }
        }).collect();

        if filtered_videos.is_empty() {
            let empty_content = if self.library_filter == LibraryFilter::Favorites {
                widget::column::with_capacity(4)
                    .spacing(10)
                    .align_x(Horizontal::Center)
                    .push(widget::icon::from_name("emblem-favorite-symbolic").size(48))
                    .push(widget::text::title3(self.language.library_favorites_empty()))
                    .push(widget::text::caption(self.language.library_favorites_empty_desc()))
                    .push(
                        widget::button::standard(self.language.library_filter_all())
                            .leading_icon(widget::icon::from_name("view-grid-symbolic"))
                            .on_press(Message::SelectLibraryFilter(LibraryFilter::All))
                    )
            } else {
                widget::column::with_capacity(2)
                    .spacing(8)
                    .align_x(Horizontal::Center)
                    .push(widget::text::title3(self.language.library_filter_empty()))
                    .push(
                        widget::button::standard(self.language.library_filter_all())
                            .leading_icon(widget::icon::from_name("view-grid-symbolic"))
                            .on_press(Message::SelectLibraryFilter(LibraryFilter::All))
                    )
            };

            let empty_filter_msg = widget::container(empty_content)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center);

            return Element::from(
                widget::column::with_capacity(2)
                    .spacing(14)
                    .push(top_section)
                    .push(empty_filter_msg)
            );
        }

        let active_paths = self.active_wallpaper_paths();

        let selected_output = self.selected_output.clone();
        let custom_videos = self.config.custom_videos.clone();
        let favorites_set = self.favorites_set.clone();
        let total_count = filtered_videos.len();
        let display_limit = self.library_limit.min(total_count);
        let has_more = total_count > display_limit;
        let displayed_videos: Vec<_> = filtered_videos.into_iter().take(display_limit).collect();
        let load_more_label = self.language.library_btn_load_more();

        let dark = cosmic::theme::is_dark();
        let grid = widget::responsive(move |size| {
            let card_w = 330.0f32;
            let gap = 18.0f32;
            let available_w = size.width.max(280.0);
            let cols = ((available_w + gap) / (card_w + gap)).floor().max(1.0) as usize;

            let mut cards_column = widget::column::with_capacity(displayed_videos.len() / cols + 2)
                .spacing(gap)
                .width(Length::Fill)
                .align_x(Horizontal::Center);

            for chunk in displayed_videos.chunks(cols) {
                let mut card_row = widget::row::with_capacity(chunk.len())
                    .spacing(gap)
                    .align_y(Alignment::Center);

                for video in chunk {
                    let path_str = video.path.to_string_lossy().to_string();
                    let is_active = active_paths.contains(path_str.as_str());

                    let mut card_content = widget::column::with_capacity(5).spacing(8).padding(12);

                    if let Some(thumb) = &video.thumb_path {
                        let mut img_btn = widget::button::image(thumb.clone())
                            .width(card_w - 24.0)
                            .selected(is_active);
                        img_btn = img_btn.on_press(Message::ApplyWallpaper {
                            video_path: video.path.clone(),
                            output: selected_output.clone(),
                        });
                        card_content = card_content.push(img_btn);
                    } else {
                        let placeholder = widget::container(widget::text::body(self.language.library_extracting_frame()))
                            .width(Length::Fixed(240.0))
                            .height(Length::Fixed(135.0))
                            .align_x(Horizontal::Center)
                            .align_y(Vertical::Center);
                        card_content = card_content.push(placeholder);
                    }

                    let title = widget::text::body(&video.name).size(14);
                    let size_lbl = widget::text::caption(&video.size_formatted);
                    card_content = card_content.push(title).push(size_lbl);

                    let apply_btn = if is_active {
                        widget::button::suggested(self.language.library_active())
                            .leading_icon(widget::icon::from_name("emblem-ok-symbolic"))
                    } else {
                        widget::button::standard(self.language.library_apply())
                            .leading_icon(widget::icon::from_name("view-fullscreen-symbolic"))
                    };
                    let apply_btn = apply_btn.on_press(Message::ApplyWallpaper {
                        video_path: video.path.clone(),
                        output: selected_output.clone(),
                    });

                    let is_fav = favorites_set.contains(path_str.as_str());
                    let fav_tooltip = if is_fav {
                        self.language.library_fav_remove()
                    } else {
                        self.language.library_fav_add()
                    };

                    let mut fav_btn = widget::button::icon(widget::icon::from_name("emblem-favorite-symbolic"))
                        .tooltip(fav_tooltip)
                        .on_press(Message::ToggleFavorite(video.path.clone()));

                    if is_fav {
                        fav_btn = fav_btn.class(cosmic::theme::Button::Suggested);
                    }

                    let mut action_row = widget::row::with_capacity(3)
                        .spacing(8)
                        .align_y(Alignment::Center)
                        .push(apply_btn)
                        .push(fav_btn);

                    // Safe delete vs remove: distinguish online download vs user file
                    if video.is_downloaded {
                        action_row = action_row.push(
                            widget::button::icon(widget::icon::from_name("user-trash-symbolic"))
                                .on_press(Message::DeleteDownloadedWallpaper(video.path.clone()))
                        );
                    } else if custom_videos.contains(&path_str) {
                        action_row = action_row.push(
                            widget::button::icon(widget::icon::from_name("list-remove-symbolic"))
                                .on_press(Message::RemoveWallpaperFromLibrary(video.path.clone()))
                        );
                    }

                    card_content = card_content.push(action_row);

                    let card_container = widget::container(card_content)
                        .width(Length::Fixed(card_w))
                        .class(crate::ui::glass_panel(dark, if dark { 0.72 } else { 0.92 }, 18.0));

                    card_row = card_row.push(card_container);
                }
                cards_column = cards_column.push(card_row);
            }

            if has_more {
                let load_more_btn = widget::button::standard(format!(
                    "{} ({} / {})",
                    load_more_label,
                    display_limit,
                    total_count
                ))
                .leading_icon(widget::icon::from_name("view-more-symbolic"))
                .on_press(Message::LoadMoreLibraryWallpapers);

                let load_more_container = widget::container(load_more_btn)
                    .width(Length::Fill)
                    .align_x(Horizontal::Center)
                    .padding(14);

                cards_column = cards_column.push(load_more_container);
            }

            cards_column.into()
        });

        let main_layout = widget::column::with_capacity(2)
            .spacing(14)
            .push(top_section)
            .push(widget::scrollable(grid).height(Length::Fill));

        Element::from(main_layout)
    }

    pub(crate) fn view_cinematic_library(&self) -> Element<'_, Message> {
        let pool = self.switcher_pool();
        let n = pool.len();
        if n == 0 {
            let empty_msg = widget::column::with_capacity(4)
                .spacing(14)
                .align_x(Horizontal::Center)
                .push(widget::text::title2(self.language.library_empty_title()))
                .push(widget::text::body(self.language.library_empty_desc()))
                .push(
                    widget::row::with_capacity(2)
                        .spacing(12)
                        .push(
                            widget::button::suggested(self.language.library_btn_add_video())
                                .leading_icon(widget::icon::from_name("list-add-symbolic"))
                                .on_press(Message::PickVideoFile)
                        )
                        .push(
                            widget::button::standard(self.language.library_btn_add_folder())
                                .leading_icon(widget::icon::from_name("folder-symbolic"))
                                .on_press(Message::PickFolder)
                        )
                );

            return widget::container(empty_msg)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center)
                .into();
        }

        let curr_idx = self.switcher_index % n;
        let selected_video = pool[curr_idx];
        let path_str = selected_video.path.to_string_lossy().to_string();
        let is_active = self.active_wallpaper_paths().contains(path_str.as_str());
        let dark = cosmic::theme::is_dark();

        // Prewarm neighbors in background
        let prev_idx = if curr_idx > 0 { curr_idx - 1 } else { n - 1 };
        let next_idx = (curr_idx + 1) % n;
        let mut prewarm_paths = Vec::new();
        if pool[prev_idx].is_video {
            prewarm_paths.push(pool[prev_idx].path.clone());
        }
        if pool[next_idx].is_video {
            prewarm_paths.push(pool[next_idx].path.clone());
        }
        if !prewarm_paths.is_empty() {
            crate::scanner::thumbs::prewarm_cinematic_video_anims(&prewarm_paths);
        }

        // 1. Top Wide Stage Player Preview Banner
        let stage_img: Element<'_, Message> = if selected_video.is_video {
            if let Some(anim_handle) = crate::scanner::thumbs::cinematic_video_anim_handle(
                &selected_video.path,
                self.cinematic_frame_tick,
            ) {
                widget::image(anim_handle)
                    .width(Length::Fill)
                    .height(Length::Fixed(380.0))
                    .content_fit(cosmic::iced::ContentFit::Contain)
                    .into()
            } else if let Some(thumb) = &selected_video.thumb_path {
                widget::image(thumb.clone())
                    .width(Length::Fill)
                    .height(Length::Fixed(380.0))
                    .content_fit(cosmic::iced::ContentFit::Contain)
                    .into()
            } else {
                let ph = widget::container(widget::text::title2("Sword Cinematic"))
                    .width(Length::Fill)
                    .height(Length::Fixed(380.0))
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Center);
                ph.into()
            }
        } else if let Some(thumb) = &selected_video.thumb_path {
            widget::image(thumb.clone())
                .width(Length::Fill)
                .height(Length::Fixed(380.0))
                .content_fit(cosmic::iced::ContentFit::Contain)
                .into()
        } else {
            let ph = widget::container(widget::text::title2("Sword Cinematic"))
                .width(Length::Fill)
                .height(Length::Fixed(380.0))
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center);
            ph.into()
        };

        let player_stage = widget::container(stage_img)
            .width(Length::Fill)
            .height(Length::Fixed(380.0))
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .class(cosmic::theme::Container::custom(|_theme| {
                cosmic::iced::widget::container::Style {
                    background: Some(cosmic::iced::Background::Color(cosmic::iced::Color::from_rgba(0.0, 0.0, 0.0, 0.70))),
                    border: cosmic::iced::Border {
                        radius: 14.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            }));

        let badge_text = if selected_video.is_video {
            format!("🎬 Video • {}", selected_video.size_formatted)
        } else {
            format!("🖼️ Imagen • {}", selected_video.size_formatted)
        };

        let status_badge = if is_active {
            widget::text::caption("● En reproducción como fondo activo")
        } else {
            widget::text::caption("○ Listo para reproducir")
        };

        let apply_btn = if is_active {
            widget::button::suggested(self.language.library_active())
                .leading_icon(widget::icon::from_name("emblem-ok-symbolic"))
        } else {
            widget::button::suggested("▶ Aplicar este fondo de pantalla")
                .leading_icon(widget::icon::from_name("media-playback-start-symbolic"))
        }
        .on_press(Message::ApplyWallpaper {
            video_path: selected_video.path.clone(),
            output: self.selected_output.clone(),
        });

        let is_fav = self.favorites_set.contains(path_str.as_str());
        let fav_btn = if is_fav {
            widget::button::suggested(self.language.library_fav_remove())
                .leading_icon(widget::icon::from_name("emblem-favorite-symbolic"))
        } else {
            widget::button::standard(self.language.library_fav_add())
                .leading_icon(widget::icon::from_name("emblem-favorite-symbolic"))
        }
        .on_press(Message::ToggleFavorite(selected_video.path.clone()));

        let open_btn = widget::button::standard(self.language.open_in_file_manager())
            .leading_icon(widget::icon::from_name("folder-open-symbolic"))
            .on_press(Message::OpenWallpapersFolder);

        let stage_info = widget::column::with_capacity(4)
            .spacing(8)
            .push(widget::text::title2(&selected_video.name))
            .push(widget::row::with_capacity(3).spacing(12).align_y(Alignment::Center)
                .push(widget::text::body(badge_text))
                .push(status_badge)
            )
            .push(
                widget::row::with_capacity(3)
                    .spacing(12)
                    .align_y(Alignment::Center)
                    .push(apply_btn)
                    .push(fav_btn)
                    .push(open_btn)
            );

        let stage_card = widget::container(
            widget::column::with_capacity(2)
                .spacing(12)
                .push(player_stage)
                .push(widget::container(stage_info).padding(14))
        )
        .width(Length::Fill)
        .class(crate::ui::glass_panel(dark, if dark { 0.72 } else { 0.92 }, 20.0));

        // 2. Bottom Rotating Carousel
        let mut carousel_row = widget::row::with_capacity(9)
            .spacing(14)
            .align_y(Alignment::Center);

        let prev_btn = widget::button::icon(widget::icon::from_name("go-previous-symbolic"))
            .on_press(Message::SwitcherPrev);
        carousel_row = carousel_row.push(prev_btn);

        let offsets = [-3, -2, -1, 0, 1, 2, 3];
        for offset in offsets {
            let item_idx = ((curr_idx as i32 + offset).rem_euclid(n as i32)) as usize;
            let item = pool[item_idx];
            let is_center = offset == 0;

            let (card_w, card_h) = if is_center {
                (210.0f32, 130.0f32)
            } else {
                (140.0f32, 88.0f32)
            };

            let card_img: Element<'_, Message> = if let Some(thumb) = &item.thumb_path {
                widget::image(thumb.clone())
                    .width(Length::Fixed(card_w))
                    .height(Length::Fixed(card_h))
                    .into()
            } else {
                widget::container(widget::icon::from_name("video-x-generic-symbolic").size(24))
                    .width(Length::Fixed(card_w))
                    .height(Length::Fixed(card_h))
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Center)
                    .class(cosmic::theme::Container::Card)
                    .into()
            };

            let card_content = if is_center {
                widget::column::with_capacity(2)
                    .spacing(6)
                    .align_x(Horizontal::Center)
                    .push(card_img)
                    .push(
                        widget::container(widget::text::caption(&item.name))
                            .padding([4, 10])
                            .class(cosmic::theme::Container::Card)
                    )
            } else {
                widget::column::with_capacity(1)
                    .align_x(Horizontal::Center)
                    .push(card_img)
            };

            let item_btn = widget::mouse_area(
                widget::container(card_content)
                    .padding(if is_center { 6 } else { 2 })
                    .class(if is_center {
                        cosmic::theme::Container::Card
                    } else {
                        cosmic::theme::Container::Transparent
                    })
            )
            .on_press(Message::SwitcherSelect(item_idx));

            carousel_row = carousel_row.push(item_btn);
        }

        let next_btn = widget::button::icon(widget::icon::from_name("go-next-symbolic"))
            .on_press(Message::SwitcherNext);
        carousel_row = carousel_row.push(next_btn);

        let carousel_container = widget::container(carousel_row)
            .width(Length::Fill)
            .align_x(Horizontal::Center)
            .padding([10, 0]);

        let cinematic_layout = widget::column::with_capacity(2)
            .spacing(16)
            .width(Length::Fill)
            .push(stage_card)
            .push(carousel_container);

        widget::scrollable(cinematic_layout)
            .height(Length::Fill)
            .into()
    }

    pub(crate) fn view_honeycomb_library(&self) -> Element<'_, Message> {
        let pool = self.switcher_pool();
        let n = pool.len();
        if n == 0 {
            let empty_text = widget::column::with_capacity(3)
                .spacing(12)
                .align_x(Horizontal::Center)
                .push(widget::text::title2(self.language.library_empty_title()))
                .push(widget::text::body("No hay fondos descargados en el panal. Descarga fondos desde Explorar o agrega videos a la biblioteca."))
                .push(
                    widget::button::suggested("Explorar Fondos")
                        .leading_icon(widget::icon::from_name("system-search-symbolic"))
                        .on_press(Message::NavigateToPage(crate::app::Page::Explore))
                );

            let empty_container = widget::container(empty_text)
                .padding(24)
                .class(cosmic::theme::Container::Card);

            let positioned = widget::container(empty_container)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center);

            return positioned.into();
        }

        let curr_idx = self.switcher_index % n;
        let accent_base = cosmic::theme::active().cosmic().accent.base;
        let accent_color: cosmic::iced::Color = accent_base.into();
        let accent_rgb: [u8; 3] = [
            (accent_color.r * 255.0).round() as u8,
            (accent_color.g * 255.0).round() as u8,
            (accent_color.b * 255.0).round() as u8,
        ];

        let responsive_honeycomb = widget::responsive(move |size| {
            crate::ui::switcher::SWITCHER_LOGICAL_WIDTH.store(size.width.to_bits(), std::sync::atomic::Ordering::Relaxed);
            crate::ui::switcher::SWITCHER_LOGICAL_HEIGHT.store(size.height.to_bits(), std::sync::atomic::Ordering::Relaxed);

            let layout = crate::ui::switcher::HoneycombLayout::new_with_scroll(size.width, size.height, n, curr_idx, self.honeycomb_scroll_x);
            let mut cards = Vec::new();

            for item_idx in 0..n {
                let (col, row) = crate::ui::switcher::panal_slot_for_index(item_idx);
                let (cx, cy) = layout.item_center(col, row);

                let card_w = layout.card_width();
                let card_h = layout.card_height();

                if cx + card_w * 0.5 < -60.0 || cx - card_w * 0.5 > size.width + 60.0 {
                    continue;
                }

                let is_active = item_idx == curr_idx;
                let draw_x = cx - card_w * 0.5;
                let draw_y = cy - card_h * 0.5;

                let video = pool[item_idx];
                let card_element: Element<'_, Message> = if video.is_video {
                    if let Some(handle) = crate::scanner::thumbs::honeycomb_anim_handle(
                        &video.path,
                        is_active,
                        Some(accent_rgb),
                        self.honeycomb_frame_tick,
                    ) {
                        widget::image(handle)
                            .width(Length::Fixed(card_w))
                            .height(Length::Fixed(card_h))
                            .into()
                    } else if let Some(handle) =
                        crate::scanner::thumbs::honeycomb_thumb_handle(&video.path, is_active, Some(accent_rgb))
                    {
                        widget::image(handle)
                            .width(Length::Fixed(card_w))
                            .height(Length::Fixed(card_h))
                            .into()
                    } else if let Some(hex_thumb) =
                        crate::scanner::thumbs::honeycomb_thumb_for_media(&video.path, is_active, Some(accent_rgb))
                    {
                        widget::image(hex_thumb)
                            .width(Length::Fixed(card_w))
                            .height(Length::Fixed(card_h))
                            .into()
                    } else if let Some(thumb) = &video.thumb_path {
                        widget::image(thumb.clone())
                            .width(Length::Fixed(card_w))
                            .height(Length::Fixed(card_h))
                            .into()
                    } else {
                        widget::container(widget::icon::from_name("video-x-generic-symbolic").size(28))
                            .width(Length::Fixed(card_w))
                            .height(Length::Fixed(card_h))
                            .align_x(Horizontal::Center)
                            .align_y(Vertical::Center)
                            .class(cosmic::theme::Container::Card)
                            .into()
                    }
                } else {
                    if let Some(handle) =
                        crate::scanner::thumbs::honeycomb_thumb_handle(&video.path, is_active, Some(accent_rgb))
                    {
                        widget::image(handle)
                            .width(Length::Fixed(card_w))
                            .height(Length::Fixed(card_h))
                            .into()
                    } else if let Some(hex_thumb) =
                        crate::scanner::thumbs::honeycomb_thumb_for_media(&video.path, is_active, Some(accent_rgb))
                    {
                        widget::image(hex_thumb)
                            .width(Length::Fixed(card_w))
                            .height(Length::Fixed(card_h))
                            .into()
                    } else if let Some(thumb) = &video.thumb_path {
                        widget::image(thumb.clone())
                            .width(Length::Fixed(card_w))
                            .height(Length::Fixed(card_h))
                            .into()
                    } else {
                        widget::container(widget::icon::from_name("image-x-generic-symbolic").size(28))
                            .width(Length::Fixed(card_w))
                            .height(Length::Fixed(card_h))
                            .align_x(Horizontal::Center)
                            .align_y(Vertical::Center)
                            .class(cosmic::theme::Container::Card)
                            .into()
                    }
                };

                let pinned = cosmic::iced::widget::pin(card_element)
                    .x(draw_x)
                    .y(draw_y);

                cards.push(Element::from(pinned));
            }

            let stack = cosmic::iced::widget::stack(cards)
                .width(Length::Fill)
                .height(Length::Fill);

            widget::container(stack)
                .width(Length::Fill)
                .height(Length::Fill)
                .class(cosmic::theme::Container::Transparent)
                .into()
        });

        widget::container(responsive_honeycomb)
            .width(Length::Fill)
            .height(Length::Fill)
            .class(cosmic::theme::Container::Transparent)
            .into()
    }
}
