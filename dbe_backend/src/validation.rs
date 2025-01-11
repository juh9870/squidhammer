use crate::etype::eitem::EItemInfo;
use crate::json_utils::repr::JsonRepr;
use crate::m_try;
use crate::registry::ETypesRegistry;
use crate::value::EValue;
use atomic_refcell::AtomicRefCell;
use diagnostic::context::DiagnosticContextMut;
use miette::{miette, Context};
use std::borrow::Cow;
use std::fmt::Debug;
use std::sync::{Arc, LazyLock};
use ustr::{Ustr, UstrMap};

pub mod ids;

static VALIDATORS: LazyLock<AtomicRefCell<UstrMap<Arc<dyn DataValidator>>>> =
    LazyLock::new(|| AtomicRefCell::new(default_validators().collect()));

fn default_validators() -> impl Iterator<Item = (Ustr, Arc<dyn DataValidator>)> {
    let v: Vec<Arc<dyn DataValidator>> =
        vec![Arc::new(ids::numeric::Id), Arc::new(ids::numeric::Ref)];
    v.into_iter().map(|item| (Ustr::from(&item.name()), item))
}

pub trait DataValidator: Send + Sync + Debug {
    fn name(&self) -> Cow<'static, str>;

    fn clear_cache(&self, registry: &ETypesRegistry);

    fn validate(
        &self,
        registry: &ETypesRegistry,
        ctx: DiagnosticContextMut,
        item: Option<&EItemInfo>,
        data: &EValue,
    ) -> miette::Result<()>;
}

#[derive(Debug, Clone)]
pub struct Validator(Arc<dyn DataValidator>);

/// Looks up the validator given their name
pub fn validator_by_name(name: Ustr) -> Option<Validator> {
    // trace!("looking up validator by name: {:?}", name);
    VALIDATORS.borrow().get(&name).map(|x| Validator(x.clone()))
}

impl DataValidator for Validator {
    fn name(&self) -> Cow<'static, str> {
        self.0.name()
    }

    fn clear_cache(&self, registry: &ETypesRegistry) {
        self.0.clear_cache(registry)
    }

    fn validate(
        &self,
        registry: &ETypesRegistry,
        ctx: DiagnosticContextMut,
        item: Option<&EItemInfo>,
        data: &EValue,
    ) -> miette::Result<()> {
        self.0.validate(registry, ctx, item, data)
    }
}

pub fn clear_validation_cache(registry: &ETypesRegistry) {
    for validator in VALIDATORS.borrow().values() {
        validator.clear_cache(registry)
    }
}

/// Validates the provided data, writing all the userspace errors to the context
///
/// This function will clear all the downstream reports in the context before
/// starting the validation process
///
/// This function will return Ok(()) unless an internal error happens, usually
/// indicating a corrupt application state
pub fn validate(
    registry: &ETypesRegistry,
    mut ctx: DiagnosticContextMut,
    item: Option<&EItemInfo>,
    data: &EValue,
) -> miette::Result<()> {
    ctx.clear_downstream();
    validate_inner(registry, ctx, item, data)
}

fn validate_inner(
    registry: &ETypesRegistry,
    mut ctx: DiagnosticContextMut,
    item: Option<&EItemInfo>,
    data: &EValue,
) -> miette::Result<()> {
    m_try(|| {
        if let Some(validators) = item.map(|i| i.validators()) {
            for validator in validators {
                validator
                    .validate(registry, ctx.enter_inline(), item, data)
                    .with_context(|| format!("validator `{}` failed", validator.name()))?;
            }
        }

        match data {
            EValue::Null => {}
            EValue::Boolean { .. } => {}
            EValue::Number { .. } => {}
            EValue::String { .. } => {}
            EValue::Struct { ident, fields } => {
                let obj = registry.get_struct(ident).ok_or_else(|| {
                    miette!(
                        "!!INTERNAL ERROR!! unknown object ID or not a struct `{}`",
                        ident
                    )
                })?;

                if let Some(repr) = &obj.repr {
                    for v in repr.validators().iter() {
                        v.validate(registry, ctx.enter_inline(), item, data)?;
                    }
                }

                let default = data.ty().default_value(registry);
                let default = default.try_as_struct().with_context(|| {
                    format!(
                        "!!INTERNAL ERROR!! bad default value for struct `{}`",
                        ident
                    )
                })?;

                for field in &obj.fields {
                    let data_field = match fields.get(&field.name) {
                        None => default.get(&field.name).ok_or_else(|| {
                            miette!(
                                "!!INTERNAL ERROR!! field `{}` is missing in struct `{}`",
                                field.name,
                                ident
                            )
                        })?,
                        Some(f) => f,
                    };

                    validate_inner(
                        registry,
                        ctx.enter_field(field.name.as_str()),
                        Some(&field.ty),
                        data_field,
                    )?;
                }
            }
            EValue::Enum { data, variant } => {
                let enum_data = registry.get_enum(&variant.enum_id()).ok_or_else(|| {
                    miette!(
                        "!!INTERNAL ERROR!! unknown enum ID or not an enum `{}`",
                        data
                    )
                })?;

                if let Some(repr) = &enum_data.repr {
                    for v in repr.validators().iter() {
                        v.validate(registry, ctx.enter_inline(), item, data)?;
                    }
                }

                validate_inner(
                    registry,
                    ctx.enter_variant(variant.variant_name().as_str()),
                    item,
                    data,
                )?;
            }
            EValue::List { values, .. } => {
                for (idx, value) in values.iter().enumerate() {
                    validate_inner(registry, ctx.enter_index(idx), None, value)?;
                }
            }
            EValue::Map { values, .. } => {
                for (idx, x) in values.values().enumerate() {
                    validate_inner(registry, ctx.enter_index(idx), None, x)?;
                }
            }
        }
        Ok(())
    })
    .with_context(|| format!("in path `{}`", ctx.path()))
}
