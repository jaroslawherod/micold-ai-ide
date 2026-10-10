//! Process-wide environment variables set for one test and put back when it ends.

use std::ffi::OsString;

/// Sets variables (`None` removes one) and restores the previous values on drop. Tests that use it
/// hold their own mutex so only one runs at a time: the environment is shared by the process.
pub struct Env {
    saved: Vec<(&'static str, Option<OsString>)>,
}

impl Env {
    pub fn set(vars: &[(&'static str, Option<OsString>)]) -> Self {
        let saved = vars
            .iter()
            .map(|(name, value)| {
                let previous = std::env::var_os(name);
                match value {
                    Some(value) => std::env::set_var(name, value),
                    None => std::env::remove_var(name),
                }
                (*name, previous)
            })
            .collect();
        Self { saved }
    }
}

impl Drop for Env {
    fn drop(&mut self) {
        for (name, previous) in self.saved.drain(..) {
            match previous {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
    }
}
