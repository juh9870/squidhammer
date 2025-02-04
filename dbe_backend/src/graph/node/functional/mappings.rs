use crate::etype::conversion::ManualEItemInfoAdapter;
use crate::etype::eitem::EItemInfo;
use crate::etype::EDataType;
use crate::graph::node::functional::{side_effects_node, CustomEValue, C};
use crate::graph::node::NodeFactory;
use crate::project::side_effects::mappings::Mappings;
use crate::project::side_effects::SideEffectsContext;
use crate::project::EXTENSION_VALUE;
use crate::registry::ETypesRegistry;
use crate::value::id::ETypeId;
use crate::value::{ENumber, EValue};
use miette::bail;
use std::sync::{Arc, LazyLock};

pub static KIND_ID: LazyLock<ETypeId> =
    LazyLock::new(|| ETypeId::from_raw("sys:mappings/kind".into()));
pub static RANGE_ID: LazyLock<ETypeId> =
    LazyLock::new(|| ETypeId::from_raw("sys:math/range".into()));

struct ERangeList;
impl ManualEItemInfoAdapter for ERangeList {
    fn edata_type(registry: &ETypesRegistry) -> EItemInfo {
        EItemInfo::simple_type(registry.list_of(EDataType::Object { ident: *RANGE_ID }))
    }
}

struct MappingsKind;
impl ManualEItemInfoAdapter for MappingsKind {
    fn edata_type(_registry: &ETypesRegistry) -> EItemInfo {
        EItemInfo::simple_type(EDataType::Object { ident: *KIND_ID })
    }
}

pub(super) fn mappings_nodes() -> Vec<Arc<dyn NodeFactory>> {
    vec![
        side_effects_node(
            |ctx: C, path: String, persistent: bool, input: String, value: ENumber| {
                let mappings = get_mappings_for_path(
                    ctx.context.registry,
                    &mut ctx.extras.side_effects,
                    None,
                    &path,
                )?;

                let id = mappings.set_id(input, value.0 as i64, persistent)?;

                Ok(ENumber::from(id as f64))
            },
            "set_mapping",
            &["path", "persistent", "input", "value"],
            &["output"],
            &["utility.mappings"],
        ),
        side_effects_node(
            |ctx: C,
             path: String,
             ranges: CustomEValue<ERangeList>,
             persistent: bool,
             kind: CustomEValue<MappingsKind>,
             value: String| {
                let kind = kind.0;

                let EValue::Enum { data, .. } = kind else {
                    bail!("kind input must be an enum, got {:?}", kind);
                };

                let kind_idx = data.try_as_number()?;

                let mappings = get_mappings_for_path(
                    ctx.context.registry,
                    &mut ctx.extras.side_effects,
                    Some(&ranges.0),
                    &path,
                )?;

                let id = match kind_idx.0 {
                    0.0 => mappings.get_id_raw(value.to_string(), persistent)?,
                    1.0 => mappings.new_id(value.to_string(), persistent)?,
                    // 2.0 => mappings.existing_id(value)?,
                    2.0 => bail!(
                        "existing ID mapping is not yet implemented, blocked due to multistage runtime"
                    ), // TODO: allow once multi-stage runtime is implemented
                    _ => bail!("invalid kind index: {}", kind_idx.0),
                };

                Ok(ENumber::from(id as f64))
            },
            "mappings",
            &["path", "default_ranges", "persistent", "kind", "input"],
            &["output"],
            &["utility.mappings"],
        ),
    ]
}

fn get_mappings_for_path<'a>(
    registry: &ETypesRegistry,
    ctx: &'a mut SideEffectsContext,
    ranges: Option<&EValue>,
    path: &str,
) -> miette::Result<&'a mut Mappings> {
    let path = path.trim();

    if path.is_empty() {
        bail!("path must not be empty");
    }

    let path = format!("{}.{}", path, EXTENSION_VALUE);

    ctx.load_mappings(registry, path.as_ref(), ranges)
}
