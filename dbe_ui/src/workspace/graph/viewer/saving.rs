use crate::workspace::graph::viewer::NodeView;
use crate::workspace::graph::GraphViewer;
use camino::Utf8PathBuf;
use dbe_backend::graph::node::saving_node::{SavingNode, SavingNodeFactory};
use dbe_backend::graph::node::{NodeFactory, SnarlNode};
use egui::Ui;
use egui_hooks::UseHookExt;
use egui_snarl::{InPin, NodeId, OutPin, Snarl};
use itertools::Itertools;
use std::ops::{Deref, DerefMut};
use ustr::Ustr;

#[derive(Debug)]
pub struct SavingNodeViewer;

impl NodeView for SavingNodeViewer {
    fn id(&self) -> Ustr {
        SavingNodeFactory.id()
    }

    fn has_body(&self, _viewer: &mut GraphViewer, _node: &SnarlNode) -> miette::Result<bool> {
        Ok(true)
    }

    fn show_body(
        &self,
        _viewer: &mut GraphViewer,
        node_id: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        ui: &mut Ui,
        _scale: f32,
        snarl: &mut Snarl<SnarlNode>,
    ) -> miette::Result<()> {
        let node = snarl[node_id]
            .downcast_mut::<SavingNode>()
            .expect("SavingNodeViewer should only be used with SavingNode");

        let mut new_path: Option<Option<Utf8PathBuf>> = None;
        match node.path.as_mut() {
            None => {
                let mut checked = false;
                ui.checkbox(&mut checked, "custom path");
                if checked {
                    new_path = Some(Some(Utf8PathBuf::new()));
                }
            }
            Some(path) => {
                let mut path_str = ui
                    .use_state(|| path.as_str().to_string(), path.clone())
                    .into_var();
                ui.vertical(|ui| {
                    let mut checked = true;
                    ui.checkbox(&mut checked, "custom path");
                    ui.horizontal(|ui| {
                        ui.label("path");
                        ui.text_edit_singleline(path_str.deref_mut())
                    });
                    ui.add_enabled_ui(path_str.deref() != path.as_str(), |ui| {
                        if ui.button("apply").clicked() {
                            let path = path_str
                                .split(['/', '\\'])
                                .map(|c| sanitise_file_name::sanitise(c).trim().to_string())
                                .filter(|c| !c.is_empty())
                                .join("/");
                            new_path = Some(Some(Utf8PathBuf::from(path)));
                        }
                    });
                    if !checked {
                        new_path = Some(None);
                    }
                });
            }
        }

        if let Some(new_path) = new_path {
            node.path = new_path;
        }

        Ok(())
    }
}
