use crate::console::Console;

pub(crate) struct Terminal {}

impl Console for Terminal {}

impl Terminal {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

unsafe impl Send for Terminal {}

unsafe impl Sync for Terminal {}
