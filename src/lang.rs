use std::borrow::Cow;
use std::fmt::Display;
use std::sync::LazyLock;

use fluent_bundle::FluentValue;
use i18n_embed::{
    DefaultLocalizer, LanguageLoader,
    fluent::{FluentLanguageLoader, fluent_language_loader},
};
use oma_console::console::StyledObject;
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

/// Values accepted as [`fl!`] message arguments.
///
/// Besides the types `i18n_embed_fl::fl!` accepts out of the box (`String`,
/// `&str`, numbers, `Cow`, `Option`), this also covers styled strings such as
/// `"name".emphasis_color()`, so call sites don't need explicit `.to_string()`.
///
/// The impls below re-implement, as a trait, the `From<...> for FluentValue`
/// conversions fluent itself provides (string, borrowed string, `Cow`, and
/// `Option` variants), so that call sites can go through [`to_fluent_value`]
/// and arbitrary `Display`/styled values can be plugged in too. See:
/// - https://github.com/projectfluent/fluent-rs/blob/fluent-bundle@0.16.0/fluent-bundle/src/types/mod.rs#L292-L326
///   (the `From<...> for FluentValue` impls)
pub trait ToFluentValue {
    fn to_fluent_value(self) -> FluentValue<'static>;
}

impl ToFluentValue for String {
    fn to_fluent_value(self) -> FluentValue<'static> {
        self.into()
    }
}

impl ToFluentValue for &str {
    fn to_fluent_value(self) -> FluentValue<'static> {
        self.to_owned().into()
    }
}

impl ToFluentValue for &String {
    fn to_fluent_value(self) -> FluentValue<'static> {
        self.clone().into()
    }
}

impl ToFluentValue for Cow<'_, str> {
    fn to_fluent_value(self) -> FluentValue<'static> {
        self.into_owned().into()
    }
}

impl<T: ToFluentValue> ToFluentValue for Option<T> {
    fn to_fluent_value(self) -> FluentValue<'static> {
        match self {
            Some(v) => v.to_fluent_value(),
            None => FluentValue::None,
        }
    }
}

// The integer conversions mirror fluent's `from_num!` macro, which generates
// `From<$int> for FluentValue` for exactly these types. See:
// - https://github.com/projectfluent/fluent-rs/blob/fluent-bundle@0.16.0/fluent-bundle/src/types/number.rs#L184-L246
//   (the `from_num!` macro and its `i8`..`usize` type list)
macro_rules! impl_to_fluent_number {
    ($($t:ty),* $(,)?) => {
        $(impl ToFluentValue for $t {
            fn to_fluent_value(self) -> FluentValue<'static> {
                self.into()
            }
        })*
    };
}

impl_to_fluent_number!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize
);

impl<D: Display> ToFluentValue for StyledObject<D> {
    fn to_fluent_value(self) -> FluentValue<'static> {
        self.to_string().into()
    }
}

impl ToFluentValue for std::path::Display<'_> {
    fn to_fluent_value(self) -> FluentValue<'static> {
        self.to_string().into()
    }
}

impl ToFluentValue for std::io::Error {
    fn to_fluent_value(self) -> FluentValue<'static> {
        self.to_string().into()
    }
}

/// Convert any [`ToFluentValue`] into a fluent message argument.
#[inline]
pub fn to_fluent_value(value: impl ToFluentValue) -> FluentValue<'static> {
    value.to_fluent_value()
}

#[macro_export]
macro_rules! fl {
    ($message_id:literal) => {{
        i18n_embed_fl::fl!($crate::lang::LANGUAGE_LOADER, $message_id)
    }};

    ($message_id:literal, $($key:ident = $value:expr),*) => {{
        i18n_embed_fl::fl!(
            $crate::lang::LANGUAGE_LOADER,
            $message_id,
            $($key = $crate::lang::to_fluent_value($value)),*
        )
    }};
}

// Get the `Localizer` to be used for localizing this library.
#[inline]
pub fn localizer() -> DefaultLocalizer<'static> {
    DefaultLocalizer::new(&*LANGUAGE_LOADER, &Localizations)
}
