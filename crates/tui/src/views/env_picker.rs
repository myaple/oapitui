/// State for the environment selector popup.
pub struct EnvPickerState {
    /// Index into Config::environments. `None` means "no environment".
    pub selected: usize,
    /// Total count including the "None" entry at index 0.
    pub count: usize,
}

impl Default for EnvPickerState {
    fn default() -> Self {
        Self {
            selected: 0,
            count: 1,
        }
    }
}

impl EnvPickerState {
    pub fn new(env_count: usize, active_env: Option<usize>) -> Self {
        Self {
            // Index 0 = "None", index 1.. = environments
            selected: active_env.map(|i| i + 1).unwrap_or(0),
            count: env_count + 1,
        }
    }

    pub fn next(&mut self) {
        if self.count > 0 {
            self.selected = (self.selected + 1).min(self.count - 1);
        }
    }

    pub fn prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Returns `None` if "None" is selected, or `Some(index)` into the environments vec.
    pub fn chosen_env(&self) -> Option<usize> {
        if self.selected == 0 {
            None
        } else {
            Some(self.selected - 1)
        }
    }
}
