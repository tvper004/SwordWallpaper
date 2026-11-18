pub mod library;
pub mod explore;
pub mod monitors;
pub mod settings;
pub mod bar;
pub mod about;
pub mod switcher;
pub mod sidebar;

use std::sync::{Arc, LazyLock};

type BackgroundHandle = cosmic::iced::widget::image::Handle;

static DARK_BACKDROP: LazyLock<BackgroundHandle> = LazyLock::new(|| {
    decode_backdrop(include_bytes!("../../resources/ui/sword-wallpaper-dark.png"))
});
static LIGHT_BACKDROP: LazyLock<BackgroundHandle> = LazyLock::new(|| {
    decode_backdrop(include_bytes!("../../resources/ui/sword-wallpaper-light.png"))
});

fn decode_backdrop(encoded: &[u8]) -> BackgroundHandle {
    let rgba = image::load_from_memory(encoded)
        .expect("bundled Sword Wallpaper backdrop must decode")
        .into_rgba8();
    let (width, height) = rgba.dimensions();
    BackgroundHandle::from_rgba(width, height, rgba.into_raw())
}

pub(crate) fn app_background(dark: bool) -> BackgroundHandle {
    if dark {
        DARK_BACKDROP.clone()
    } else {
        LIGHT_BACKDROP.clone()
    }
}

pub(crate) fn sword_theme(dark: bool) -> cosmic::theme::Theme {
    let builder = if dark {
        cosmic::cosmic_theme::ThemeBuilder::dark()
    } else {
        cosmic::cosmic_theme::ThemeBuilder::light()
    };
    let accent = cosmic::cosmic_theme::palette::Srgb::new(0.03, 0.58, 0.98);
    cosmic::theme::Theme::custom(Arc::new(builder.accent(accent).build()))
}

#[cfg(test)]
mod tests {
    use super::sword_theme;

    #[test]
    fn theme_tracks_requested_system_appearance() {
        assert!(!sword_theme(false).theme_type.is_dark());
        assert!(sword_theme(true).theme_type.is_dark());
    }
}

pub(crate) fn glass_panel(
    dark: bool,
    opacity: f32,
    radius: f32,
) -> cosmic::theme::Container<'static> {
    let (background, border, text) = if dark {
        (
            cosmic::iced::Color::from_rgba(0.015, 0.035, 0.075, opacity),
            cosmic::iced::Color::from_rgba(0.24, 0.68, 1.0, 0.34),
            cosmic::iced::Color::from_rgb(0.93, 0.96, 1.0),
        )
    } else {
        (
            cosmic::iced::Color::from_rgba(0.99, 0.99, 1.0, opacity),
            cosmic::iced::Color::from_rgba(0.25, 0.50, 0.78, 0.22),
            cosmic::iced::Color::from_rgb(0.12, 0.16, 0.23),
        )
    };

    cosmic::theme::Container::custom(move |_| cosmic::iced::widget::container::Style {
        background: Some(cosmic::iced::Background::Color(background)),
        text_color: Some(text),
        icon_color: Some(text),
        border: cosmic::iced::Border {
            color: border,
            width: 1.0,
            radius: radius.into(),
        },
        ..Default::default()
    })
}
