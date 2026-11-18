use crate::app::{AuraApp, Message};
use cosmic::iced::alignment::Horizontal;
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;

impl AuraApp {
    pub(crate) fn view_about(&self) -> Element<'_, Message> {
        let logo = widget::icon::from_svg_bytes(
            include_bytes!("../../resources/icons/hicolor/scalable/apps/io.github.antwny.aura.svg"),
        )
        .icon()
        .size(88);

        let heading = widget::column::with_capacity(3)
            .spacing(7)
            .align_x(Horizontal::Center)
            .push(widget::text::title1("Sword Wallpaper"))
            .push(widget::text::body(self.language.about_tagline()))
            .push(widget::text::caption(self.language.about_version_info()));

        let details = widget::column::with_capacity(2)
            .spacing(14)
            .padding(20)
            .width(Length::Fixed(540.0))
            .push(
                widget::row::with_capacity(2)
                    .spacing(20)
                    .align_y(Alignment::Center)
                    .push(widget::text::body(self.language.about_developer_lbl()).width(Length::Fixed(170.0)))
                    .push(widget::text::body("ChOe")),
            )
            .push(
                widget::row::with_capacity(2)
                    .spacing(20)
                    .align_y(Alignment::Center)
                    .push(widget::text::body(self.language.about_engine_lbl()).width(Length::Fixed(170.0)))
                    .push(widget::text::body(self.language.bar_wayland_tag())),
            );

        let content = widget::column::with_capacity(3)
            .spacing(22)
            .padding([38, 24])
            .align_x(Horizontal::Center)
            .width(Length::Fill)
            .push(logo)
            .push(heading)
            .push(widget::container(details));

        Element::from(
            widget::container(widget::scrollable(content))
                .height(Length::Fill)
                .width(Length::Fill)
                .align_x(Horizontal::Center),
        )
    }
}
