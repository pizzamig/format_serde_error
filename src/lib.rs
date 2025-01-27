//! Format serde errors in a way to make it obvious where the error in the
//! source file was.
//!
//! Example:
//!
//! ```rust
//! use format_serde_error::SerdeError;
//!
//! #[derive(Debug, serde::Serialize, serde::Deserialize)]
//! struct Config {
//!     values: Vec<String>,
//! }
//! # fn main() {
//! #   if let Err(err) = parse_config() {
//! #     eprintln!("{}", err)
//! #   }
//! # }
//!
//! fn parse_config() -> Result<Config, anyhow::Error> {
//!   let config_str = "values:
//!   - 'first'
//!   - 'second'
//!   - third:";
//!
//!   let config = serde_yaml::from_str::<Config>(config_str)
//!     .map_err(|err| SerdeError::new(config_str.to_string(), err))?;
//!
//!   Ok(config)
//! }
//! ```
//!
//! The output will be:
//! ```text
//! Error:
//!    | values:
//!    |   - 'first'
//!    |   - 'second'
//!  4 |   - third:
//!    |           ^ values[2]: invalid type: map, expected a string at line 4 column 10
//! ```

#![deny(missing_docs)]
#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]
#![warn(clippy::unwrap_used)]
#![warn(rust_2018_idioms, unused_lifetimes, missing_debug_implementations)]

use colored::Colorize;
use std::fmt;

mod control;

#[cfg(test)]
mod test;

pub use control::{
    always_color,
    never_color,
    set_coloring_mode,
    use_environment,
    ColoringMode,
};

/// Amount of lines to show before and after the line containing the error.
pub const CONTEXT_LINES: usize = 3;

/// Struct for formatting the error together with the source file to give a
/// nicer output.
#[derive(Debug)]
pub struct SerdeError {
    input: String,
    message: String,
    line: Option<usize>,
    column: Option<usize>,
}

/// Supported error types by the crate. Currently only [`serde_yaml`] and
/// [`serde_json`] are supported.
#[derive(Debug)]
pub enum ErrorTypes {
    /// Contains [`serde_yaml::Error`].
    Yaml(serde_yaml::Error),

    /// Contains [`serde_json::Error`].
    Json(serde_json::Error),
}

impl std::error::Error for SerdeError {}

impl fmt::Display for SerdeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.format(f)
    }
}

impl From<serde_yaml::Error> for ErrorTypes {
    fn from(err: serde_yaml::Error) -> Self {
        Self::Yaml(err)
    }
}

impl From<serde_json::Error> for ErrorTypes {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err)
    }
}

impl SerdeError {
    /// Create a new [`SerdeError`] from compatible serde errors. See
    /// [`ErrorTypes`] for more information.
    pub fn new(input: String, err: impl Into<ErrorTypes>) -> SerdeError {
        let error = err.into();

        let (message, line, column) = match error {
            ErrorTypes::Yaml(e) => match e.location() {
                // Don't set line/column if we don't have a location
                None => (e.to_string(), None, None),

                Some(location) => (
                    e.to_string(),
                    Some(location.line()),
                    Some(location.column()),
                ),
            },

            ErrorTypes::Json(e) => (e.to_string(), Some(e.line()), Some(e.column())),
        };

        Self {
            input,
            message,
            line,
            column,
        }
    }

    fn format(&self, f: &mut fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        // If line and column are not set we assume that we can't make a nice output
        // so we will just print the original message in red and bold
        if self.line.is_none() && self.column.is_none() {
            return writeln!(f, "{}", self.message.red().bold());
        }

        let error_line = self.line.unwrap_or_default();
        let error_column = self.column.unwrap_or_default();

        // Amount of lines to show before and after the error line
        let context_lines = CONTEXT_LINES;

        // Skip until we are amount of context lines before the error line (context)
        // plus the line with the error ( + 1)
        // Saturating sub if the error is in the first few line we can't take more
        // context
        let skip = usize::saturating_sub(error_line, context_lines + 1);

        // Take lines before and after (context * 2) plus the line with the error ( + 1)
        let take = context_lines * 2 + 1;

        // Minimize the input to only what we need so we can reuse it without
        // having to iterate over the whole input again.
        let minimized_input = self.input.lines().skip(skip).take(take).collect::<Vec<_>>();

        // If the minimized_input is empty we can assume that the input was empty as
        // well. In that case we can't make a nice output so we will just print
        // the original message in red and bold
        if minimized_input.is_empty() {
            return writeln!(f, "{}", self.message.red().bold());
        }

        // To reduce the amount of space text takes we want to remove unnecessary
        // whitespace in front of the text.
        // Find the line with the least amount of whitespace in front and use
        // that to remove the whitespace later.
        // We basically want to find the least indented line.
        // We cant just use trim as that would remove all whitespace and remove all
        // indentation.
        let whitespace_count = minimized_input
            .iter()
            .map(|line| line.chars().take_while(|s| s.is_whitespace()).count())
            .min()
            .unwrap_or_default();

        let separator = " | ".blue().bold();

        // When we don't print the line_position we want to fill up the space not used
        // by the line_position with whitespace instead
        let fill_line_position = format!("{: >fill$}", "", fill = error_line.to_string().len());

        // Want to avoid printing when we are not at the beginning of the line. For
        // example anyhow will write 'Error:' in front of the output before
        // printing the buffer
        writeln!(f)?;

        self.input
            .lines()
            .into_iter()
            .enumerate()
            .skip(skip)
            .take(take)
            .map(|(index, text)| {
                // Make the index start at 1 makes it nicer to work with
                // Also remove unnecessary whitespace in front of text
                (
                    index + 1,
                    text.chars().skip(whitespace_count).collect::<String>(),
                )
            })
            .try_for_each(|(line_position, text)| {
                self.format_line(
                    f,
                    line_position,
                    error_line,
                    error_column,
                    &text,
                    whitespace_count,
                    &separator,
                    &fill_line_position,
                )
            })?;

        Ok(())
    }

    // TODO: Maybe make another internal struct for formatting instead of having
    // this list of args.
    #[allow(clippy::too_many_arguments)]
    fn format_line(
        &self,
        f: &mut fmt::Formatter<'_>,
        line_position: usize,
        error_line: usize,
        error_column: usize,
        text: &str,
        whitespace_count: usize,
        separator: &colored::ColoredString,
        fill_line_position: &str,
    ) -> Result<(), std::fmt::Error> {
        if line_position == error_line {
            Self::format_error_line(f, text, line_position, separator)?;

            self.format_error_information(
                f,
                whitespace_count,
                separator,
                fill_line_position,
                error_column,
            )
        } else {
            Self::format_context_line(f, text, separator, fill_line_position)
        }
    }

    fn format_error_line(
        f: &mut fmt::Formatter<'_>,
        text: &str,
        line_position: usize,
        separator: &colored::ColoredString,
    ) -> Result<(), std::fmt::Error> {
        writeln!(
            f,
            " {}{}{}",
            line_position.to_string().blue().bold(),
            separator,
            text
        )
    }

    fn format_error_information(
        &self,
        f: &mut fmt::Formatter<'_>,
        whitespace_count: usize,
        separator: &colored::ColoredString,
        fill_line_position: &str,
        error_column: usize,
    ) -> Result<(), std::fmt::Error> {
        // Print whitespace until we reach the column value of the message. We also
        // have to add the amount of whitespace in front of the other lines.
        let fill_column_position = format!(
            "{: >column$}^ {}",
            "",
            self.message,
            column = error_column - whitespace_count
        );

        writeln!(
            f,
            " {}{}{}",
            fill_line_position,
            separator,
            fill_column_position.red().bold(),
        )
    }

    fn format_context_line(
        f: &mut fmt::Formatter<'_>,
        text: &str,
        separator: &colored::ColoredString,
        fill_line_position: &str,
    ) -> Result<(), std::fmt::Error> {
        writeln!(f, " {}{}{}", fill_line_position, separator, text.yellow())
    }
}
