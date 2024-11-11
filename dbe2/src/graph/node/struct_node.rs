use crate::etype::eitem::EItemInfo;
use crate::etype::EDataType;
use crate::graph::node::{impl_serde_node, InputData, Node, NodeFactory, OutputData, SnarlNode};
use crate::registry::ETypesRegistry;
use crate::value::id::ETypeId;
use crate::value::EValue;
use miette::miette;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ustr::Ustr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructNode {
    pub id: ETypeId,
}

impl StructNode {
    pub fn new(id: ETypeId) -> Self {
        Self { id }
    }
}

impl Node for StructNode {
    impl_serde_node!();

    fn id(&self) -> Ustr {
        "struct_node".into()
    }

    fn title(&self) -> String {
        self.id.to_string()
    }

    fn inputs_count(&self, registry: &ETypesRegistry) -> usize {
        let Some(data) = registry.get_struct(&self.id) else {
            return 0;
        };

        data.fields.len()
    }

    fn input_unchecked(
        &self,
        registry: &ETypesRegistry,
        input: usize,
    ) -> miette::Result<InputData> {
        let Some(data) = registry.get_struct(&self.id) else {
            panic!("Unknown struct")
        };

        let field = &data.fields[input];

        Ok(InputData {
            ty: field.ty.clone(),
            name: field.name,
        })
    }

    fn outputs_count(&self, registry: &ETypesRegistry) -> usize {
        let Some(_) = registry.get_struct(&self.id) else {
            return 0;
        };
        1
    }

    fn output_unchecked(
        &self,
        registry: &ETypesRegistry,
        output: usize,
    ) -> miette::Result<OutputData> {
        let Some(_) = registry.get_struct(&self.id) else {
            panic!("Unknown struct")
        };

        if output != 0 {
            panic!("Struct only has one output")
        }

        Ok(OutputData {
            ty: EItemInfo::simple_type(EDataType::Object { ident: self.id }),
            name: "output".into(),
        })
    }

    fn execute(
        &self,
        registry: &ETypesRegistry,
        inputs: &[EValue],
        outputs: &mut Vec<EValue>,
    ) -> miette::Result<()> {
        let data = registry
            .get_struct(&self.id)
            .ok_or_else(|| miette!("unknown struct `{}`", self.id))?;

        let mut fields = BTreeMap::default();

        for (i, field) in data.fields.iter().enumerate() {
            assert_eq!(inputs[i].ty(), field.ty.ty());
            fields.insert(field.name, inputs[i].clone());
        }

        outputs.clear();
        outputs.push(EValue::Struct {
            fields,
            ident: self.id,
        });

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct StructNodeFactory;

impl NodeFactory for StructNodeFactory {
    fn id(&self) -> Ustr {
        "struct_node".into()
    }

    fn categories(&self) -> &'static [&'static str] {
        &[]
    }

    fn create(&self) -> SnarlNode {
        Box::new(StructNode {
            id: ETypeId::temp(0),
        })
    }
}
