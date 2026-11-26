//! Pieces the Neutrino pages share: the error bar that turns server errors into
//! sentences, and the notice that says what Neutrino keeps about each action.

mod error_bar;
mod keeps_notice;

pub use error_bar::{
    error_copy, error_copy_text, ErrorAction, ErrorBar, ErrorCopy, ErrorCopyBar, ErrorTone,
    RequestAccessButton,
};
pub use keeps_notice::{KeepsDialog, KeepsNotice, KeepsWhat};
