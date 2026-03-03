use std::cell::Cell;

pub struct ServerListState {
    pub selected: usize,
    /// Height of the visible list area — updated by the UI renderer each frame.
    pub page_size: Cell<u16>,
}

impl Default for ServerListState {
    fn default() -> Self {
        Self {
            selected: 0,
            page_size: Cell::new(10),
        }
    }
}
