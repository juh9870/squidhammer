use crate::etype::eenum::variant::{EEnumVariant, EEnumVariantId};
use crate::etype::eenum::EEnumData;
use crate::etype::eitem::EItemInfo;
use crate::etype::EDataType;
use crate::graph::node::commands::{SnarlCommand, SnarlCommands};
use crate::graph::node::{impl_serde_node, InputData, Node, NodeFactory, OutputData, SnarlNode};
use crate::registry::ETypesRegistry;
use crate::value::EValue;
use egui_snarl::{InPinId, NodeId, OutPinId};
use serde::{Deserialize, Serialize};
use ustr::Ustr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumNode {
    variant: Option<EEnumVariantId>,
}

impl EnumNode {
    pub fn new(variant: EEnumVariantId) -> Self {
        Self {
            variant: Some(variant),
        }
    }

    fn get_data<'a>(
        &self,
        registry: &'a ETypesRegistry,
    ) -> Option<(&'a EEnumData, &'a EEnumVariant)> {
        let variant = self.variant.as_ref()?;

        variant.enum_variant(registry)
    }

    pub fn variant(&self) -> EEnumVariantId {
        self.variant
            .expect("Variant should be set before using EnumNode")
    }

    pub fn set_variant(
        &mut self,
        commands: &mut SnarlCommands,
        node: NodeId,
        variant: EEnumVariantId,
    ) -> miette::Result<()> {
        if variant == self.variant() {
            return Ok(());
        }

        self.variant = Some(variant);
        commands.push(SnarlCommand::ReconnectInput {
            id: InPinId { node, input: 0 },
        });
        commands.push(SnarlCommand::ReconnectOutput {
            id: OutPinId { node, output: 0 },
        });

        Ok(())
    }
}

impl Node for EnumNode {
    impl_serde_node!();
    fn id(&self) -> Ustr {
        EnumNodeFactory.id()
    }

    fn inputs_count(&self, registry: &ETypesRegistry) -> usize {
        let Some((_, _)) = self.get_data(registry) else {
            return 0;
        };
        1
    }

    fn input_unchecked(
        &self,
        registry: &ETypesRegistry,
        input: usize,
    ) -> miette::Result<InputData> {
        let Some((_, variant)) = self.get_data(registry) else {
            panic!("Unknown enum variant");
        };
        if input != 0 {
            panic!("Enum only has one input");
        }
        Ok(InputData {
            ty: variant.data.clone(),
            name: variant.name,
        })
    }

    fn outputs_count(&self, registry: &ETypesRegistry) -> usize {
        let Some((_, _)) = self.get_data(registry) else {
            return 0;
        };
        1
    }

    fn output_unchecked(
        &self,
        registry: &ETypesRegistry,
        output: usize,
    ) -> miette::Result<OutputData> {
        let Some((data, _)) = self.get_data(registry) else {
            panic!("Unknown enum variant");
        };

        if output != 0 {
            panic!("Enum only has one output");
        }

        Ok(OutputData {
            ty: EItemInfo::simple_type(EDataType::Object { ident: data.ident }),
            name: "output".into(),
        })
    }

    fn execute(
        &self,
        registry: &ETypesRegistry,
        inputs: &[EValue],
        outputs: &mut Vec<EValue>,
    ) -> miette::Result<()> {
        let Some((data, variant)) = self.get_data(registry) else {
            panic!("Unknown enum variant");
        };

        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0].ty(), variant.data.ty());

        outputs.clear();
        outputs.push(EValue::Enum {
            variant: self.variant(),
            data: Box::new(inputs[0].clone()),
        });
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct EnumNodeFactory;

impl NodeFactory for EnumNodeFactory {
    fn id(&self) -> Ustr {
        "enum_node".into()
    }

    fn categories(&self) -> &'static [&'static str] {
        &[]
    }

    fn create(&self) -> SnarlNode {
        Box::new(EnumNode { variant: None })
    }
}
