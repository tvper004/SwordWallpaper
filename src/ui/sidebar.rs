use crate::app::{AuraApp, LibraryFilter, Message, Page};
use cosmic::iced::alignment::Horizontal;
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;

fn entry<'a>(label: &'a str, icon: &'static str, active: bool, message: Message) -> Element<'a, Message> {
    let body = widget::column::with_capacity(2)
        .spacing(4)
        .align_x(Alignment::Center)
        .push(widget::icon::from_name(icon).size(23))
        .push(widget::text::caption(label));
    let body = widget::container(body)
        .width(Length::Fill)
        .height(Length::Fixed(62.0))
        .align_x(Horizontal::Center)
        .align_y(Alignment::Center);

    let class = if active {
        cosmic::theme::Button::Suggested
    } else {
        cosmic::theme::Button::Standard
    };

    widget::button::custom(body)
        .class(class)
        .width(Length::Fill)
        .on_press(message)
        .into()
}

impl AuraApp {
    pub(crate) fn view_navigation_sidebar(&self) -> Element<'_, Message> {
        let on_library = self.active_page == Page::Library;
        let mut primary = widget::column::with_capacity(5).spacing(7);
        primary = primary
            .push(entry(
                self.language.nav_home(),
                "go-home-symbolic",
                on_library && self.library_filter == LibraryFilter::All,
                Message::NavigateToLibraryFilter(LibraryFilter::All),
            ))
            .push(entry(
                self.language.nav_explore(),
                "web-browser-symbolic",
                self.active_page == Page::Explore,
                Message::NavigateToPage(Page::Explore),
            ))
            .push(entry(
                self.language.library_filter_live(),
                "video-x-generic-symbolic",
                on_library && self.library_filter == LibraryFilter::Live,
                Message::NavigateToLibraryFilter(LibraryFilter::Live),
            ))
            .push(entry(
                self.language.library_filter_favorites(),
                "emblem-favorite-symbolic",
                on_library && self.library_filter == LibraryFilter::Favorites,
                Message::NavigateToLibraryFilter(LibraryFilter::Favorites),
            ))
            .push(entry(
                self.language.nav_monitors(),
                "video-display-symbolic",
                self.active_page == Page::Monitors,
                Message::NavigateToPage(Page::Monitors),
            ));

        let secondary = widget::column::with_capacity(2)
            .spacing(7)
            .push(entry(
                self.language.nav_settings(),
                "preferences-system-symbolic",
                self.active_page == Page::Settings,
                Message::NavigateToPage(Page::Settings),
            ))
            .push(entry(
                self.language.nav_about(),
                "help-about-symbolic",
                self.active_page == Page::About,
                Message::NavigateToPage(Page::About),
            ));

        let dark = cosmic::theme::is_dark();
        let nav = widget::column::with_capacity(3)
            .spacing(10)
            .push(primary)
            .push(widget::Space::new().height(Length::Fill))
            .push(secondary);

        Element::from(
            widget::container(nav)
                .width(Length::Fixed(112.0))
                .height(Length::Fill)
                .padding([12, 8])
                .class(crate::ui::glass_panel(dark, if dark { 0.82 } else { 0.84 }, 24.0)),
        )
    }
}
