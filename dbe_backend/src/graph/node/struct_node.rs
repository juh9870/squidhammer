use crate::etype::eitem::EItemInfo;
use crate::etype::eobject::EObject;
use crate::etype::estruct::EStructField;
use crate::etype::EDataType;
use crate::graph::node::commands::SnarlCommands;
use crate::graph::node::ports::fields::{
    get_field, map_inputs, sync_fields, FieldMapper, IoDirection,
};
use crate::graph::node::ports::NodePortType;
use crate::graph::node::{
    impl_serde_node, ExecutionExtras, InputData, Node, NodeContext, NodeFactory, OutputData,
};
use crate::project::docs::{Docs, DocsRef};
use crate::value::id::ETypeId;
use crate::value::EValue;
use egui_snarl::NodeId;
use miette::miette;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ustr::Ustr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructNode {
    pub id: ETypeId,
    #[serde(default)]
    pub fields: Vec<Ustr>,
}

struct StructNodeFieldMapper;

impl FieldMapper for StructNodeFieldMapper {
    type Field = EStructField;
    type Local = Ustr;
    type Type = ();

    fn matches(&self, field: &Self::Field, local: &Self::Local) -> bool {
        &field.name == local
    }

    fn to_local(&self, field: &Self::Field) -> Self::Local {
        field.name
    }
}

impl StructNode {
    pub fn new(id: ETypeId) -> Self {
        Self { id, fields: vec![] }
    }
}

impl Node for StructNode {
    impl_serde_node!();

    fn id(&self) -> Ustr {
        "struct_node".into()
    }

    fn title(&self, context: NodeContext, _docs: &Docs) -> String {
        let Some(data) = context.registry.get_struct(&self.id) else {
            return format!("Unknown struct `{}`", self.id);
        };

        data.title(context.registry)
    }

    fn update_state(
        &mut self,
        context: NodeContext,
        commands: &mut SnarlCommands,
        id: NodeId,
    ) -> miette::Result<()> {
        let Some(data) = context.registry.get_struct(&self.id) else {
            return Ok(());
        };

        sync_fields(
            &StructNodeFieldMapper,
            commands,
            &data.fields,
            &mut self.fields,
            id,
            IoDirection::Input,
        );

        Ok(())
    }

    fn inputs_count(&self, context: NodeContext) -> usize {
        let Some(data) = context.registry.get_struct(&self.id) else {
            return 0;
        };

        data.fields.len()
    }

    fn input_unchecked(&self, context: NodeContext, input: usize) -> miette::Result<InputData> {
        let Some(data) = context.registry.get_struct(&self.id) else {
            panic!("Unknown struct")
        };

        let field = get_field(&StructNodeFieldMapper, &data.fields, &self.fields, input);
        if let Some(field) = field {
            Ok(InputData::new(field.ty.clone().into(), field.name)
                .with_custom_docs(DocsRef::TypeField(self.id, field.name)))
        } else {
            Ok(InputData::new(NodePortType::Invalid, self.fields[input]))
        }
    }

    fn outputs_count(&self, context: NodeContext) -> usize {
        let Some(_) = context.registry.get_struct(&self.id) else {
            return 0;
        };
        1
    }

    fn output_unchecked(&self, context: NodeContext, output: usize) -> miette::Result<OutputData> {
        let Some(_) = context.registry.get_struct(&self.id) else {
            panic!("Unknown struct")
        };

        if output != 0 {
            panic!("Struct only has one output")
        }

        Ok(OutputData::new(
            EItemInfo::simple_type(EDataType::Object { ident: self.id }).into(),
            "output".into(),
        ))
    }

    fn execute(
        &self,
        context: NodeContext,
        inputs: &[EValue],
        outputs: &mut Vec<EValue>,
        _variables: &mut ExecutionExtras,
    ) -> miette::Result<()> {
        let data = context
            .registry
            .get_struct(&self.id)
            .ok_or_else(|| miette!("unknown struct `{}`", self.id))?;

        let mut struct_fields = vec![];

        map_inputs(
            &StructNodeFieldMapper,
            context.registry,
            &data.fields,
            &self.fields,
            inputs,
            &mut struct_fields,
        )?;

        let mut fields = BTreeMap::default();

        for (i, field) in data.fields.iter().enumerate() {
            // assert_eq!(inputs[i].ty(), field.ty.ty());
            fields.insert(field.name, struct_fields[i].clone());
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

    fn create(&self) -> Box<dyn Node> {
        Box::new(StructNode {
            id: ETypeId::temp(0),
            fields: vec![],
        })
    }
}
