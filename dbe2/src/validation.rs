use crate::etype::eitem::EItemInfo;
use crate::m_try;
use crate::registry::ETypesRegistry;
use crate::value::EValue;
use atomic_refcell::AtomicRefCell;
use diagnostic::context::DiagnosticContextRef;
use miette::{miette, Context};
use std::borrow::Cow;
use std::fmt::Debug;
use std::sync::{Arc, LazyLock};
use ustr::{Ustr, UstrMap};

static VALIDATORS: LazyLock<AtomicRefCell<UstrMap<Arc<dyn DataValidator>>>> =
    LazyLock::new(|| AtomicRefCell::new(default_validators().collect()));

fn default_validators() -> impl Iterator<Item = (Ustr, Arc<dyn DataValidator>)> {
    let v: Vec<(Ustr, Arc<dyn DataValidator>)> = vec![];
    v.into_iter()
}
pub trait DataValidator: Send + Sync + Debug {
    fn name(&self) -> Cow<'static, str>;

    fn validate(
        &self,
        registry: &ETypesRegistry,
        ctx: DiagnosticContextRef,
        item: Option<&EItemInfo>,
        data: &EValue,
    ) -> miette::Result<()>;
}

#[derive(Debug, Clone)]
pub struct Validator(Arc<dyn DataValidator>);

impl DataValidator for Validator {
    fn name(&self) -> Cow<'static, str> {
        self.0.name()
    }

    fn validate(
        &self,
        registry: &ETypesRegistry,
        ctx: DiagnosticContextRef,
        item: Option<&EItemInfo>,
        data: &EValue,
    ) -> miette::Result<()> {
        self.0.validate(registry, ctx, item, data)
    }
}

pub fn validate(
    registry: &ETypesRegistry,
    mut ctx: DiagnosticContextRef,
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

                    validate(
                        registry,
                        ctx.enter_field(field.name.as_str()),
                        Some(&field.ty),
                        data_field,
                    )?;
                }
            }
            EValue::Enum { data, variant } => {
                validate(
                    registry,
                    ctx.enter_variant(variant.variant_name().as_str()),
                    item,
                    data,
                )?;
            }
            EValue::List { values, .. } => {
                for (idx, value) in values.iter().enumerate() {
                    validate(registry, ctx.enter_index(idx), None, value)?;
                }
            }
            EValue::Map { values, .. } => {
                for (idx, x) in values.values().enumerate() {
                    validate(registry, ctx.enter_index(idx), None, x)?;
                }
            }
        }
        Ok(())
    })
    .with_context(|| format!("in path `{}`", ctx.path()))
}
