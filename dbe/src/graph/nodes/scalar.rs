use crate::graph::commands::Command;
use crate::value::ENumber;
use node_macro::editor_node;
use smallvec::SmallVec;

#[editor_node(name = ScalarMake, outputs = [result], categories = ["scalar"])]
pub fn scalar_make(value: ENumber) -> ENumber {
    value
}

#[editor_node(name = ScalarAdd, outputs = [result], categories = ["scalar"])]
pub fn scalar_add(values: SmallVec<[ENumber; 2]>) -> ENumber {
    values.iter().sum()
}

#[editor_node(name = ScalarSub, outputs = [result], categories = ["scalar"])]
pub fn scalar_sub(a: ENumber, b: ENumber) -> ENumber {
    a - b
}

#[editor_node(name = ScalarMult, outputs = [result], categories = ["scalar"])]
pub fn scalar_mult(values: SmallVec<[ENumber; 2]>) -> ENumber {
    values.iter().product()
}

#[editor_node(name = ScalarDiv, outputs = [result], categories = ["scalar"])]
pub fn scalar_div(a: ENumber, b: ENumber) -> ENumber {
    a / b
}

#[editor_node(name = ScalarPrint, outputs = [], categories = ["scalar"])]
pub fn scalar_print(commands: &mut Vec<Command>, item: ENumber) {
    commands.push(Command::Println(format!("{item}")));
}
