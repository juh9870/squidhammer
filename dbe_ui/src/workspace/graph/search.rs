use crate::ui_props::PROP_OBJECT_GRAPH_SEARCH_HIDE;
use dbe_backend::etype::eobject::EObject;
use dbe_backend::etype::EDataType;
use dbe_backend::graph::editing::GraphEditingContext;
use dbe_backend::graph::node::commands::SnarlCommands;
use dbe_backend::graph::node::ports::InputData;
use dbe_backend::graph::node::{all_node_factories, node_factories_by_category};
use dbe_backend::project::project_graph::ProjectGraphs;
use dbe_backend::registry::ETypesRegistry;
use dbe_backend::value::id::{EListId, ETypeId};
use egui::{Pos2, Ui};
use egui_hooks::UseHookExt;
use egui_snarl::{InPinId, NodeId, OutPinId};
use itertools::{Itertools, Position};
use nucleo::pattern::{CaseMatching, Normalization};
use nucleo::{Item, Nucleo, Snapshot};
use smallvec::SmallVec;
use std::collections::BTreeMap;
use std::hash::Hash;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use ustr::Ustr;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum NodeCombo {
    Factory(Ustr),
    Subgraph(Uuid, String),
    Object(ETypeId),
    List(EListId),
}

impl AsRef<str> for NodeCombo {
    fn as_ref(&self) -> &str {
        match self {
            NodeCombo::Factory(id) => id.as_str(),
            NodeCombo::Object(id) => id.as_raw().unwrap(),
            NodeCombo::Subgraph(_, id) => id.as_str(),
            NodeCombo::List(id) => id.as_raw().unwrap(),
        }
    }
}

impl NodeCombo {
    pub fn create(
        &self,
        ctx: &mut GraphEditingContext,
        pos: Pos2,
    ) -> miette::Result<SmallVec<[NodeId; 2]>> {
        match self {
            NodeCombo::Factory(id) => ctx.create_node(*id, pos),
            NodeCombo::Object(id) => ctx.create_object_node(*id, pos, None),
            NodeCombo::Subgraph(id, _) => ctx.create_subgraph_node(*id, pos),
            NodeCombo::List(id) => ctx.create_list_node(*id, pos),
        }
    }

    pub fn create_from_pin(
        &self,
        ctx: &mut GraphEditingContext,
        pos: Pos2,
        pin: &InPinId,
        commands: &mut SnarlCommands,
    ) -> miette::Result<SmallVec<[NodeId; 2]>> {
        let in_pin = ctx.snarl.in_pin(*pin);
        let nodes = match self {
            NodeCombo::Factory(id) => ctx.create_node(*id, pos)?,
            NodeCombo::Subgraph(id, _) => ctx.create_subgraph_node(*id, pos)?,
            NodeCombo::Object(ident) => {
                let inline_value = ctx.inline_values.remove(pin);
                ctx.create_object_node(*ident, pos, inline_value)?
            }
            NodeCombo::List(id) => ctx.create_list_node(*id, pos)?,
        };
        if let Some(node_id) = nodes.last() {
            let node = &ctx.snarl[*node_id];
            let out_count = node.outputs_count(ctx.as_node_context());
            for output in 0..out_count {
                let out_pin = &ctx.snarl.out_pin(OutPinId {
                    node: *node_id,
                    output,
                });
                if ctx.connect(out_pin, &in_pin, commands)? {
                    break;
                }
            }
        }

        Ok(nodes)
    }
}

pub fn category_tree(
    graphs: Option<&ProjectGraphs>,
    registry: &ETypesRegistry,
) -> BTreeMap<String, Vec<NodeCombo>> {
    let mut categories: BTreeMap<String, Vec<NodeCombo>> = node_factories_by_category()
        .iter()
        .map(|(category, factories)| {
            (
                category.to_string(),
                factories
                    .iter()
                    .map(|f| NodeCombo::Factory(f.id()))
                    .collect_vec(),
            )
        })
        .collect();

    if let Some(graphs) = graphs {
        for (id, graph) in &graphs.graphs {
            if !graph.is_node_group {
                continue;
            }

            for category in &graph.categories {
                categories
                    .entry(category.clone())
                    .or_default()
                    .push(NodeCombo::Subgraph(*id, graph.name.clone()));
            }
        }
    }

    for obj in registry
        .all_ready_objects()
        .filter(|obj| !PROP_OBJECT_GRAPH_SEARCH_HIDE.get(obj.extra_properties(), false))
    {
        let id = obj.ident();
        let category = ["types"]
            .into_iter()
            .chain(id.as_raw().unwrap().split([':', '/', '.']))
            .with_position()
            .take_while(|x| x.0 != Position::Last)
            .map(|x| x.1)
            .join(".");

        categories
            .entry(category)
            .or_default()
            .push(NodeCombo::Object(id));
    }

    categories
}

#[derive(Clone)]
pub struct GraphSearch(Arc<parking_lot::RwLock<SearchData>>);

struct SearchData {
    engine: Nucleo<NodeCombo>,
    last_query: Option<String>,
}

impl GraphSearch {
    pub fn empty() -> Self {
        let nucleo = init_nucleo();
        let data = SearchData {
            engine: nucleo,
            last_query: None,
        };

        GraphSearch(Arc::new(parking_lot::RwLock::new(data)))
    }

    pub fn all_nodes(
        graphs: Option<&ProjectGraphs>,
        registry: &ETypesRegistry,
        filter: impl Fn(&NodeCombo) -> bool,
    ) -> GraphSearch {
        let nucleo = init_nucleo();

        let injector = nucleo.injector();
        let factories = all_node_factories();
        let all_nodes = factories.iter().map(|(id, _)| NodeCombo::Factory(*id));

        if let Some(graphs) = graphs {
            for node in graphs
                .graphs
                .iter()
                .filter(|x| x.1.is_node_group)
                .map(|x| NodeCombo::Subgraph(*x.0, x.1.name.clone()))
                .filter(|x| filter(x))
            {
                push_to_injector(&injector, node);
            }
        }

        let objects = registry
            .all_ready_objects()
            .filter(|obj| !PROP_OBJECT_GRAPH_SEARCH_HIDE.get(obj.extra_properties(), false))
            .map(|s| NodeCombo::Object(s.ident()));
        for node in all_nodes.chain(objects).filter(|x| filter(x)) {
            push_to_injector(&injector, node);
        }

        let data = SearchData {
            engine: nucleo,
            last_query: None,
        };

        GraphSearch(Arc::new(parking_lot::RwLock::new(data)))
    }

    pub fn for_input_data(input: &InputData) -> Self {
        if !input.ty.is_specific() {
            return Self::empty();
        }

        let nucleo = init_nucleo();
        let injector = nucleo.injector();

        match input.ty.ty() {
            EDataType::Object { ident } => {
                push_to_injector(&injector, NodeCombo::Object(ident));
            }
            EDataType::List { id } => {
                push_to_injector(&injector, NodeCombo::List(id));
            }
            _ => {
                // TODO: Support other types
            }
        };

        let data = SearchData {
            engine: nucleo,
            last_query: None,
        };

        GraphSearch(Arc::new(parking_lot::RwLock::new(data)))
    }

    pub fn tick(&self, timeout: u64) {
        self.0.write().engine.tick(timeout);
    }

    pub fn apply_search(&self, search_query: &str) {
        let mut data = self.0.write();
        let trimmed = search_query.trim();
        if data.last_query.as_ref().is_some_and(|q| q == trimmed) {
            return;
        }

        data.last_query = Some(trimmed.to_string());

        data.engine.pattern.reparse(
            0,
            search_query.trim(),
            CaseMatching::Ignore,
            Normalization::Smart,
            false,
        )
    }

    pub fn snapshot(&self) -> impl SearchSnapshot + '_ {
        parking_lot::RwLockReadGuard::map(self.0.read(), |guard| guard.engine.snapshot())
    }
}

pub fn search_ui(
    ui: &mut Ui,
    id: impl Hash,
    search: GraphSearch,
    no_search_ui: impl FnOnce(&mut Ui) -> Option<NodeCombo>,
) -> Option<NodeCombo> {
    dyn_search_ui(ui, id, search, Some(Box::new(no_search_ui)))
}

pub fn search_ui_always(ui: &mut Ui, id: impl Hash, search: GraphSearch) -> Option<NodeCombo> {
    dyn_search_ui(ui, id, search, None)
}

fn dyn_search_ui<'c>(
    ui: &mut Ui,
    id: impl Hash,
    search: GraphSearch,
    no_search_ui: Option<Box<dyn FnOnce(&mut Ui) -> Option<NodeCombo> + 'c>>,
) -> Option<NodeCombo> {
    ui.push_id(id, |ui| {
        search.tick(10);

        let mut search_query = ui.use_state(|| "".to_string(), ()).into_var();
        let search_bar = ui.text_edit_singleline(search_query.deref_mut());
        ui.use_effect(
            || {
                search_bar.request_focus();
            },
            (),
        );

        ui.use_effect(
            || {
                search.apply_search(&search_query);
            },
            search_query.trim().to_owned(),
        );

        let snapshot = search.snapshot();

        if search_query.trim() == "" {
            if let Some(no_search_ui) = no_search_ui {
                return no_search_ui(ui);
            }
        }

        for node in snapshot.matched_items(10) {
            let node = node.data;
            let name = node.as_ref();
            if ui.button(name).clicked() {
                return Some(node.clone());
            }
        }

        None
    })
    .inner
}

pub trait SearchSnapshot {
    fn matched_item_count(&self) -> u32;

    fn matched_items(
        &self,
        limit: u32,
    ) -> impl ExactSizeIterator<Item = Item<'_, NodeCombo>> + DoubleEndedIterator + '_;
}

impl<'a> SearchSnapshot for parking_lot::MappedRwLockReadGuard<'a, Snapshot<NodeCombo>> {
    fn matched_item_count(&self) -> u32 {
        Snapshot::matched_item_count(self)
    }

    fn matched_items(
        &self,
        limit: u32,
    ) -> impl ExactSizeIterator<Item = Item<'_, NodeCombo>> + DoubleEndedIterator + '_ {
        self.deref()
            .matched_items(0..self.matched_item_count().min(limit))
    }
}

fn init_nucleo() -> Nucleo<NodeCombo> {
    let mut cfg = nucleo::Config::DEFAULT;
    cfg.set_match_paths();
    Nucleo::<NodeCombo>::new(cfg, Arc::new(|| {}), None, 1)
}

fn push_to_injector(injector: &nucleo::Injector<NodeCombo>, node: NodeCombo) {
    injector.push(node, |i, col| {
        col[0] = i.as_ref().to_string().into();
    });
}
