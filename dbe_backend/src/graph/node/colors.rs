use egui_colors::{Colorix, Theme};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone)]
pub struct NodeColorScheme {
    pub theme: Box<Colorix>,
    pub dark_mode: bool,
}

impl PartialEq for NodeColorScheme {
    fn eq(&self, other: &Self) -> bool {
        self.theme.theme() == other.theme.theme() && self.dark_mode == other.dark_mode
    }
}

impl Eq for NodeColorScheme {}

impl Hash for NodeColorScheme {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for c in self.theme.theme() {
            c.rgb().hash(state);
        }
        self.dark_mode.hash(state);
    }
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
            theme: Box::new(Colorix::local_from_style(self.theme, self.dark_mode)),
            dark_mode: self.dark_mode,
        }
    }
}
