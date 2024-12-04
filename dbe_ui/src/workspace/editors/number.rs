use crate::ui_props::{PROP_FIELD_LOGARITHMIC, PROP_FIELD_MAX, PROP_FIELD_MIN};
use crate::workspace::editors::utils::{labeled_field, unsupported, EditorSize};
use crate::workspace::editors::{cast_props, DynProps, Editor, EditorProps, EditorResponse};
use dbe_backend::diagnostic::context::DiagnosticContextRef;
use dbe_backend::etype::eitem::EItemInfo;
use dbe_backend::registry::ETypesRegistry;
use dbe_backend::value::{ENumber, EValue};
use egui::{DragValue, Slider, Ui};
use num_traits::Float;
use std::ops::RangeInclusive;

#[derive(Debug)]
pub struct NumberEditor {
    slider: bool,
}

impl NumberEditor {
    pub fn new(slider: bool) -> Self {
        Self { slider }
    }
}

impl Editor for NumberEditor {
    fn props(&self, _reg: &ETypesRegistry, item: Option<&EItemInfo>) -> miette::Result<DynProps> {
        let props = item.map(|i| i.extra_properties());
        let min = props.and_then(|p| PROP_FIELD_MIN.try_get(p));
        let max = props.and_then(|p| PROP_FIELD_MAX.try_get(p));
        let logarithmic = props.and_then(|p| PROP_FIELD_LOGARITHMIC.try_get(p));

        let min = min.unwrap_or(ENumber::min_value()).0;
        let max = max.unwrap_or(ENumber::max_value()).0;

        Ok(NumericProps {
            range: min..=max,
            logarithmic: logarithmic.unwrap_or(max - min >= 1e6),
        }
        .pack())
    }

    fn size(&self, _props: &DynProps) -> EditorSize {
        EditorSize::Inline
    }

    fn edit(
        &self,
        ui: &mut Ui,
        _reg: &ETypesRegistry,
        _diagnostics: DiagnosticContextRef,
        field_name: &str,
        value: &mut EValue,
        props: &DynProps,
    ) -> EditorResponse {
        let Ok(value) = value.try_as_number_mut() else {
            unsupported!(ui, field_name, value, self);
        };

        let props = cast_props::<NumericProps>(props);

        let changed = labeled_field(ui, field_name, |ui| {
            if self.slider {
                ui.add(
                    Slider::new(&mut value.0, props.range.clone()).logarithmic(props.logarithmic),
                )
                .changed()
            } else {
                ui.add(DragValue::new(&mut value.0).range(props.range.clone()))
                    .changed()
            }
        });

        EditorResponse::new(changed.inner)
    }
}

#[derive(Debug, Clone)]
struct NumericProps {
    range: RangeInclusive<f64>,
    logarithmic: bool,
}

impl EditorProps for NumericProps {}
