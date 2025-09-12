use std::sync::LazyLock;

use i18n_embed::{
    DefaultLocalizer, LanguageLoader,
    fluent::{FluentLanguageLoader, fluent_language_loader},
};
use rust_embed::RustEmbed;

pub const DEFAULT_LANGUAGE: &str = "en_US";
pub static SYSTEM_LANG: LazyLock<String> = LazyLock::new(|| {
    let lang = LANGUAGE_LOADER.current_language();
    let mut res = lang.language.to_string();

    if let Some(region) = lang.region {
        res.push('_');
        res.push_str(&region.as_str().to_ascii_uppercase());
    }

    res
});

#[derive(RustEmbed)]
#[folder = "./i18n/"]
struct Localizations;

pub static LANGUAGE_LOADER: LazyLock<FluentLanguageLoader> = LazyLock::new(|| {
    let loader: FluentLanguageLoader = fluent_language_loader!();

    loader
        .load_fallback_language(&Localizations)
        .expect("Error while loading fallback language");

    loader
});

#[macro_export]
macro_rules! fl {
    ($message_id:literal) => {{
        i18n_embed_fl::fl!($crate::lang::LANGUAGE_LOADER, $message_id)
    }};

    ($message_id:literal, $($args:expr),*) => {{
        i18n_embed_fl::fl!($crate::lang::LANGUAGE_LOADER, $message_id, $($args), *)
    }};
}

// Get the `Localizer` to be used for localizing this library.
#[inline]
pub fn localizer() -> DefaultLocalizer<'static> {
    DefaultLocalizer::new(&*LANGUAGE_LOADER, &Localizations)
}
