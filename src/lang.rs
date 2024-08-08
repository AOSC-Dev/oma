use i18n_embed::{
    fluent::{fluent_language_loader, FluentLanguageLoader},
    unic_langid::LanguageIdentifier,
    DesktopLanguageRequester, LanguageLoader,
};
use once_cell::sync::Lazy;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "./i18n/"]
struct Localizations;

pub static LANGUAGE_LOADER: Lazy<FluentLanguageLoader> = Lazy::new(|| {
    let language_loader: FluentLanguageLoader = fluent_language_loader!();
    let requested_languages = DesktopLanguageRequester::requested_languages();
    let fallback_language: Vec<LanguageIdentifier> = vec!["en-US".parse().unwrap()];
    let languages: Vec<LanguageIdentifier> = requested_languages
        .into_iter()
        .chain(fallback_language.into_iter())
        .collect();
    language_loader
        .load_languages(&Localizations, &languages)
        .expect("Unable to local i18n languager");
    // Windows Terminal doesn't support bidirectional (BiDi) text, and renders the isolate characters incorrectly.
    // This is a temporary workaround for https://github.com/microsoft/terminal/issues/16574
    // TODO: this might break BiDi text, though we don't support any writing system depends on that.
    language_loader.set_use_isolating(false);
    language_loader
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
