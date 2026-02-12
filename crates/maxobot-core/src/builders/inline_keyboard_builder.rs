//! Inline keyboard builder with structural validation.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Maximum number of rows in one inline keyboard.
pub const MAX_KEYBOARD_ROWS: usize = 8;
/// Maximum number of buttons per row.
pub const MAX_BUTTONS_PER_ROW: usize = 8;

/// Typed inline keyboard payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct InlineKeyboard {
    /// Keyboard rows.
    pub rows: Vec<InlineKeyboardRow>,
}

/// One inline keyboard row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct InlineKeyboardRow {
    /// Row buttons.
    pub buttons: Vec<InlineKeyboardButton>,
}

/// Button action type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
pub enum InlineKeyboardButtonAction {
    /// Callback payload action.
    Callback(String),
    /// Open URL action.
    Url(String),
}

/// Inline keyboard button model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InlineKeyboardButton {
    /// Button label.
    pub text: String,
    /// Button action.
    pub action: InlineKeyboardButtonAction,
}

/// Keyboard builder failures.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum InlineKeyboardError {
    /// Row count exceeds limit.
    #[error("keyboard rows exceed limit {MAX_KEYBOARD_ROWS}")]
    TooManyRows,
    /// Button count exceeds row limit.
    #[error("keyboard row buttons exceed limit {MAX_BUTTONS_PER_ROW}")]
    TooManyButtonsInRow,
    /// Button text is empty.
    #[error("button text must not be empty")]
    EmptyButtonText,
    /// Callback payload is empty.
    #[error("callback payload must not be empty")]
    EmptyCallbackPayload,
    /// URL is empty.
    #[error("button URL must not be empty")]
    EmptyUrl,
}

/// Fluent keyboard builder.
#[derive(Debug, Clone, Default)]
pub struct InlineKeyboardBuilder {
    rows: Vec<InlineKeyboardRow>,
}

impl InlineKeyboardBuilder {
    /// Creates empty keyboard builder.
    #[must_use]
    pub fn new() -> Self {
        Self { rows: Vec::new() }
    }

    /// Starts a new row.
    pub fn begin_row(&mut self) -> Result<&mut Self, InlineKeyboardError> {
        if self.rows.len() >= MAX_KEYBOARD_ROWS {
            return Err(InlineKeyboardError::TooManyRows);
        }
        self.rows.push(InlineKeyboardRow {
            buttons: Vec::new(),
        });
        Ok(self)
    }

    /// Adds callback button to current row.
    pub fn callback_button(
        &mut self,
        text: impl Into<String>,
        payload: impl Into<String>,
    ) -> Result<&mut Self, InlineKeyboardError> {
        let text = text.into();
        let payload = payload.into();
        if text.trim().is_empty() {
            return Err(InlineKeyboardError::EmptyButtonText);
        }
        if payload.trim().is_empty() {
            return Err(InlineKeyboardError::EmptyCallbackPayload);
        }

        self.push_button(InlineKeyboardButton {
            text,
            action: InlineKeyboardButtonAction::Callback(payload),
        })
    }

    /// Adds URL button to current row.
    pub fn url_button(
        &mut self,
        text: impl Into<String>,
        url: impl Into<String>,
    ) -> Result<&mut Self, InlineKeyboardError> {
        let text = text.into();
        let url = url.into();
        if text.trim().is_empty() {
            return Err(InlineKeyboardError::EmptyButtonText);
        }
        if url.trim().is_empty() {
            return Err(InlineKeyboardError::EmptyUrl);
        }

        self.push_button(InlineKeyboardButton {
            text,
            action: InlineKeyboardButtonAction::Url(url),
        })
    }

    /// Returns built keyboard payload.
    #[must_use]
    pub fn build(self) -> InlineKeyboard {
        InlineKeyboard { rows: self.rows }
    }

    fn push_button(
        &mut self,
        button: InlineKeyboardButton,
    ) -> Result<&mut Self, InlineKeyboardError> {
        if self.rows.is_empty() {
            self.begin_row()?;
        }
        let row = self
            .rows
            .last_mut()
            .expect("at least one row exists after begin_row");
        if row.buttons.len() >= MAX_BUTTONS_PER_ROW {
            return Err(InlineKeyboardError::TooManyButtonsInRow);
        }
        row.buttons.push(button);
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::{InlineKeyboardBuilder, InlineKeyboardError, MAX_BUTTONS_PER_ROW};

    #[test]
    fn builds_keyboard_with_callback_and_url() {
        let mut builder = InlineKeyboardBuilder::new();
        builder
            .callback_button("A", "payload")
            .expect("callback should be valid")
            .url_button("B", "https://example.test")
            .expect("url should be valid");

        let keyboard = builder.build();
        assert_eq!(keyboard.rows.len(), 1);
        assert_eq!(keyboard.rows[0].buttons.len(), 2);
    }

    #[test]
    fn enforces_row_button_limit() {
        let mut builder = InlineKeyboardBuilder::new();
        for idx in 0..MAX_BUTTONS_PER_ROW {
            builder
                .callback_button(format!("B{idx}"), format!("p{idx}"))
                .expect("button should fit in row");
        }

        let error = builder
            .callback_button("overflow", "payload")
            .expect_err("row limit should be enforced");
        assert_eq!(error, InlineKeyboardError::TooManyButtonsInRow);
    }
}
