use std::error::Error;

use clap::builder::StyledStr;
use clap::builder::Styles;
use clap::builder::styling::Style;
use clap::error::ContextKind;
use clap::error::ContextValue;
use clap::error::ErrorFormatter;
use clap::error::ErrorKind;

use crate::fl;

const TAB: &str = "  ";

/// Richly formatted error context
///
/// This follows the [rustc diagnostic style guide](https://rustc-dev-guide.rust-lang.org/diagnostics.html#suggestion-style-guide).
#[non_exhaustive]
pub struct OmaClapRichFormatter;

impl ErrorFormatter for OmaClapRichFormatter {
    fn format_error(error: &clap::error::Error<Self>) -> StyledStr {
        use std::fmt::Write as _;
        let styles = &Styles::default();
        let valid = &styles.get_valid();

        let mut styled = StyledStr::new();
        start_error(&mut styled, styles);

        if !write_dynamic_context(error, &mut styled, styles) {
            if error.kind().as_str().is_some() {
                styled.push_str(&translation_errorkind(error));
            } else if let Some(source) = error.source() {
                let _ = write!(styled, "{source}");
            } else {
                styled.push_str("unknown cause");
            }
        }

        let mut suggested = false;
        if let Some(valid) = error.get(ContextKind::SuggestedSubcommand) {
            styled.push_str("\n");
            if !suggested {
                styled.push_str("\n");
                suggested = true;
            }
            did_you_mean(&mut styled, styles, &fl!("clap-subcommand-context"), valid);
        }
        if let Some(valid) = error.get(ContextKind::SuggestedArg) {
            styled.push_str("\n");
            if !suggested {
                styled.push_str("\n");
                suggested = true;
            }
            did_you_mean(&mut styled, styles, &fl!("clap-argument-context"), valid);
        }
        if let Some(valid) = error.get(ContextKind::SuggestedValue) {
            styled.push_str("\n");
            if !suggested {
                styled.push_str("\n");
                suggested = true;
            }
            did_you_mean(&mut styled, styles, "value", valid);
        }
        let suggestions = error.get(ContextKind::Suggested);
        if let Some(ContextValue::StyledStrs(suggestions)) = suggestions {
            if !suggested {
                styled.push_str("\n");
            }
            for suggestion in suggestions {
                let _ = write!(
                    styled,
                    "\n{TAB}{valid}{}{valid:#} ",
                    fl!("clap-tip-heading")
                );
                styled.push_str(&suggestion.ansi().to_string());
            }
        }

        let usage = error.get(ContextKind::Usage);
        if let Some(ContextValue::StyledStr(usage)) = usage {
            put_usage(&mut styled, usage);
        }

        try_help(&mut styled, styles, Some("--help"));

        styled
    }
}

fn translation_errorkind(error: &clap::error::Error<OmaClapRichFormatter>) -> String {
    match error.kind() {
        ErrorKind::InvalidValue => fl!("clap-errorkind-invalidvalue"),
        ErrorKind::UnknownArgument => fl!("clap-errorkind-unknown-arg"),
        ErrorKind::InvalidSubcommand => fl!("clap-errorkind-invalid-subcmd"),
        ErrorKind::NoEquals => fl!("clap-errorkind-noeq"),
        ErrorKind::ValueValidation => fl!("clap-errorkind-value-validation"),
        ErrorKind::TooManyValues => fl!("clap-errorkind-too-many-values"),
        ErrorKind::TooFewValues => fl!("clap-errorkind-too-few-values"),
        ErrorKind::WrongNumberOfValues => fl!("clap-errorkind-wrong-number-of-values"),
        ErrorKind::ArgumentConflict => fl!("clap-errorkind-arg-conflict"),
        ErrorKind::MissingRequiredArgument => {
            fl!("clap-errorkind-missing-required-arg")
        }
        ErrorKind::MissingSubcommand => fl!("clap-errorkind-missing-subcmd"),
        ErrorKind::InvalidUtf8 => fl!("clap-errorkind-invalid-utf8"),
        _ => unreachable!(),
    }
}

fn start_error(styled: &mut StyledStr, styles: &Styles) {
    use std::fmt::Write as _;
    let error = &styles.get_error();
    let _ = write!(styled, "{error}{}:{error:#} ", fl!("clap-error-heading"));
}

#[must_use]
fn write_dynamic_context(
    error: &clap::error::Error<OmaClapRichFormatter>,
    styled: &mut StyledStr,
    styles: &Styles,
) -> bool {
    use std::fmt::Write as _;
    let valid = styles.get_valid();
    let invalid = styles.get_invalid();
    let literal = styles.get_literal();

    match error.kind() {
        ErrorKind::ArgumentConflict => {
            let mut prior_arg = error.get(ContextKind::PriorArg);
            if let Some(ContextValue::String(invalid_arg)) = error.get(ContextKind::InvalidArg) {
                let arg = format!("{invalid}{invalid_arg}{invalid:#}");
                if Some(&ContextValue::String(invalid_arg.clone())) == prior_arg {
                    prior_arg = None;
                    let _ = write!(
                        styled,
                        "{}",
                        fl!("clap-dyn-errorkind-multipletimes", arg = arg),
                    );
                } else {
                    let _ = write!(
                        styled,
                        "{}",
                        fl!("clap-dyn-errorkind-argconflict", arg = arg),
                    );
                }
            } else if let Some(ContextValue::String(invalid_arg)) =
                error.get(ContextKind::InvalidSubcommand)
            {
                let arg = format!("{invalid}{invalid_arg}{invalid:#}");
                let _ = write!(
                    styled,
                    "{}",
                    fl!("clap-dyn-errorkind-subcmd-conflict", arg = arg),
                );
            } else {
                styled.push_str(&translation_errorkind(error));
            }

            if let Some(prior_arg) = prior_arg {
                match prior_arg {
                    ContextValue::Strings(values) => {
                        styled.push_str(":");
                        for v in values {
                            let _ = write!(styled, "\n{TAB}{invalid}{v}{invalid:#}",);
                        }
                    }
                    ContextValue::String(value) => {
                        let _ = write!(styled, " '{invalid}{value}{invalid:#}'");
                    }
                    _ => {
                        let _ = write!(styled, "{}", fl!("clap-dyn-errorkind-conflict-other"));
                    }
                }
            }

            true
        }
        ErrorKind::NoEquals => {
            let invalid_arg = error.get(ContextKind::InvalidArg);
            if let Some(ContextValue::String(invalid_arg)) = invalid_arg {
                let arg = format!("{invalid}{invalid_arg}{invalid:#}'");
                let _ = write!(styled, "{}", fl!("clap-dyn-errorkind-no-eq", arg = arg));
                true
            } else {
                false
            }
        }
        ErrorKind::InvalidValue => {
            let invalid_arg = error.get(ContextKind::InvalidArg);
            let invalid_value = error.get(ContextKind::InvalidValue);
            if let (
                Some(ContextValue::String(invalid_arg)),
                Some(ContextValue::String(invalid_value)),
            ) = (invalid_arg, invalid_value)
            {
                let arg = format!("{invalid}{invalid_arg}{invalid:#}");
                if invalid_value.is_empty() {
                    let _ = write!(
                        styled,
                        "{}",
                        fl!("clap-dyn-errorkind-required-arg-but-none", arg = arg),
                    );
                } else {
                    let value = format!("{invalid}{invalid_value}{invalid:#}");
                    let _ = write!(
                        styled,
                        "{}",
                        fl!(
                            "clap-dyn-errorkind-invalid-value-for-arg",
                            value = value,
                            arg = arg
                        ),
                    );
                }

                let values = error.get(ContextKind::ValidValue);
                write_values_list("possible values", styled, valid, values);

                true
            } else {
                false
            }
        }
        ErrorKind::InvalidSubcommand => {
            let invalid_sub = error.get(ContextKind::InvalidSubcommand);
            if let Some(ContextValue::String(invalid_sub)) = invalid_sub {
                let sub = format!("{invalid}{invalid_sub}{invalid:#}");
                let _ = write!(
                    styled,
                    "{}",
                    fl!("clap-dyn-errorkind-unrecognized-subcmd", sub = sub),
                );
                true
            } else {
                false
            }
        }
        ErrorKind::MissingRequiredArgument => {
            let invalid_arg = error.get(ContextKind::InvalidArg);
            if let Some(ContextValue::Strings(invalid_arg)) = invalid_arg {
                let _ = write!(styled, "{}", fl!("clap-dyn-errorkind-not-provided"));
                for v in invalid_arg {
                    let _ = write!(styled, "\n{TAB}{valid}{v}{valid:#}",);
                }
                true
            } else {
                false
            }
        }
        ErrorKind::MissingSubcommand => {
            let invalid_sub = error.get(ContextKind::InvalidSubcommand);
            if let Some(ContextValue::String(invalid_sub)) = invalid_sub {
                let sub = format!("{invalid}{invalid_sub}{invalid:#}");
                let _ = write!(
                    styled,
                    "{}",
                    fl!("clap-dyn-errorkind-subcmd-not-provided", sub = sub),
                );
                let values = error.get(ContextKind::ValidSubcommand);
                write_values_list("subcommands", styled, valid, values);

                true
            } else {
                false
            }
        }
        ErrorKind::InvalidUtf8 => false,
        ErrorKind::TooManyValues => {
            let invalid_arg = error.get(ContextKind::InvalidArg);
            let invalid_value = error.get(ContextKind::InvalidValue);
            if let (
                Some(ContextValue::String(invalid_arg)),
                Some(ContextValue::String(invalid_value)),
            ) = (invalid_arg, invalid_value)
            {
                let value = format!("{invalid}{invalid_value}{invalid:#}");
                let arg = format!("{literal}{invalid_arg}{literal:#}");
                let _ = write!(
                    styled,
                    "{}",
                    fl!(
                        "clap-dyn-errorkind-too-many-values-no-more-expected",
                        value = value,
                        arg = arg
                    )
                );
                true
            } else {
                false
            }
        }
        ErrorKind::TooFewValues => {
            let invalid_arg = error.get(ContextKind::InvalidArg);
            let actual_num_values = error.get(ContextKind::ActualNumValues);
            let min_values = error.get(ContextKind::MinValues);
            if let (
                Some(ContextValue::String(invalid_arg)),
                Some(ContextValue::Number(actual_num_values)),
                Some(ContextValue::Number(min_values)),
            ) = (invalid_arg, actual_num_values, min_values)
            {
                let were_provided = singular_or_plural(*actual_num_values as usize);
                let _ = write!(
                    styled,
                    "{valid}{min_values}{valid:#} values required by '{literal}{invalid_arg}{literal:#}'; only {invalid}{actual_num_values}{invalid:#}{were_provided}",
                );
                true
            } else {
                false
            }
        }
        ErrorKind::ValueValidation => {
            let invalid_arg = error.get(ContextKind::InvalidArg);
            let invalid_value = error.get(ContextKind::InvalidValue);
            if let (
                Some(ContextValue::String(invalid_arg)),
                Some(ContextValue::String(invalid_value)),
            ) = (invalid_arg, invalid_value)
            {
                let _ = write!(
                    styled,
                    "invalid value '{invalid}{invalid_value}{invalid:#}' for '{literal}{invalid_arg}{literal:#}'",
                );
                if let Some(source) = error.source() {
                    let _ = write!(styled, ": {source}");
                }
                true
            } else {
                false
            }
        }
        ErrorKind::WrongNumberOfValues => {
            let invalid_arg = error.get(ContextKind::InvalidArg);
            let actual_num_values = error.get(ContextKind::ActualNumValues);
            let num_values = error.get(ContextKind::ExpectedNumValues);
            if let (
                Some(ContextValue::String(invalid_arg)),
                Some(ContextValue::Number(actual_num_values)),
                Some(ContextValue::Number(num_values)),
            ) = (invalid_arg, actual_num_values, num_values)
            {
                let were_provided = singular_or_plural(*actual_num_values as usize);
                let _ = write!(
                    styled,
                    "{valid}{num_values}{valid:#} values required for '{literal}{invalid_arg}{literal:#}' but {invalid}{actual_num_values}{invalid:#}{were_provided}",
                );
                true
            } else {
                false
            }
        }
        ErrorKind::UnknownArgument => {
            let invalid_arg = error.get(ContextKind::InvalidArg);
            if let Some(ContextValue::String(invalid_arg)) = invalid_arg {
                let arg = format!("{invalid}{invalid_arg}{invalid:#}");
                let _ = write!(
                    styled,
                    "{}",
                    fl!("clap-dyn-errorkind-unexpected-arg", arg = arg),
                );
                true
            } else {
                false
            }
        }
        ErrorKind::DisplayHelp
        | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
        | ErrorKind::DisplayVersion
        | ErrorKind::Io
        | ErrorKind::Format => false,
        _ => false,
    }
}

fn write_values_list(
    list_name: &'static str,
    styled: &mut StyledStr,
    valid: &Style,
    possible_values: Option<&ContextValue>,
) {
    use std::fmt::Write as _;
    if let Some(ContextValue::Strings(possible_values)) = possible_values
        && !possible_values.is_empty()
    {
        let _ = write!(styled, "\n{TAB}[{list_name}: ");

        for (idx, val) in possible_values.iter().enumerate() {
            if idx > 0 {
                styled.push_str(", ");
            }
            let _ = write!(styled, "{valid}{}{valid:#}", Escape(val));
        }

        styled.push_str("]");
    }
}

/// Returns the singular or plural form on the verb to be based on the argument's value.
fn singular_or_plural(n: usize) -> &'static str {
    if n > 1 {
        " were provided"
    } else {
        " was provided"
    }
}

fn put_usage(styled: &mut StyledStr, usage: &StyledStr) {
    styled.push_str("\n\n");
    styled.push_str(
        &usage
            .ansi()
            .to_string()
            .replace("Usage:", &fl!("clap-usage-heading")),
    );
}

fn try_help(styled: &mut StyledStr, styles: &Styles, help: Option<&str>) {
    if let Some(help) = help {
        use std::fmt::Write as _;
        let literal = &styles.get_literal();
        let help = format!("{literal}{help}{literal:#}");
        let _ = write!(styled, "\n\n{}\n", fl!("clap-help-tips", help = help));
    } else {
        styled.push_str("\n");
    }
}

fn did_you_mean(styled: &mut StyledStr, styles: &Styles, context: &str, possibles: &ContextValue) {
    use std::fmt::Write as _;

    let valid = &styles.get_valid();
    let _ = write!(styled, "{TAB}{valid}{}:{valid:#}", fl!("clap-tip-heading"));
    if let ContextValue::String(possible) = possibles {
        let possible = format!("{valid}{possible}{valid:#}");
        let _ = write!(
            styled,
            " {}",
            fl!(
                "clap-similar-exists-single",
                possible = possible,
                context = context
            )
        );
    } else if let ContextValue::Strings(possibles) = possibles {
        if possibles.len() == 1 {
            let possible = &possibles[0];
            let _ = write!(
                styled,
                " {}",
                fl!(
                    "clap-similar-exists-single",
                    possible = possible,
                    context = context
                )
            );
        } else {
            let _ = write!(
                styled,
                " {} ",
                fl!("clap-similar-exists-multi", context = context)
            );
        }
        for (i, possible) in possibles.iter().enumerate() {
            if i != 0 {
                styled.push_str(", ");
            }
            let _ = write!(styled, "'{valid}{possible}{valid:#}'",);
        }
    }
}

struct Escape<'s>(&'s str);

impl std::fmt::Display for Escape<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.contains(char::is_whitespace) {
            std::fmt::Debug::fmt(self.0, f)
        } else {
            self.0.fmt(f)
        }
    }
}
