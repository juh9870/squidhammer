use egui_colors::{Colorix, Theme};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct NodeColorScheme {
    pub theme: Box<Colorix>,
    pub dark_mode: bool,
}

impl NodeColorScheme {
    pub(crate) fn pack(&self) -> PackedNodeColorScheme {
        PackedNodeColorScheme {
            theme: *self.theme.theme(),
            dark_mode: self.dark_mode,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackedNodeColorScheme {
    pub theme: Theme,
    pub dark_mode: bool,
}

impl PackedNodeColorScheme {
    pub(crate) fn unpack(self) -> NodeColorScheme {
        NodeColorScheme {
            theme: Box::new(Colorix::init_with_dark_mode(self.theme, self.dark_mode)),
            dark_mode: self.dark_mode,
        }
    }
}
