use crate::graph::commands::Command;
use crate::value::{ENumber, EVector2};
use nalgebra::vector;
use node_macro::editor_node;
use smallvec::SmallVec;

#[editor_node(name = Vec2Make, outputs = [result], categories = ["vec2"])]
pub fn vec2_make(x: ENumber, y: ENumber) -> EVector2 {
    vector![x, y]
}

#[editor_node(name = Vec2Add, outputs = [result], categories = ["vec2"])]
pub fn vec2_add(values: SmallVec<[EVector2; 2]>) -> EVector2 {
    values.iter().sum()
}

#[editor_node(name = Vec2Sub, outputs = [result], categories = ["vec2"])]
pub fn vec2_sub(a: EVector2, b: EVector2) -> EVector2 {
    a - b
}

#[editor_node(name = Vec2Scale, outputs = [result], categories = ["vec2"])]
pub fn vec2_scale(vec: EVector2, scale: ENumber) -> EVector2 {
    vec * scale
}

#[editor_node(name = Vec2Print, outputs = [], categories = ["vec2"])]
pub fn vec2_print(commands: &mut Vec<Command>, item: EVector2) {
    commands.push(Command::Println(format!(
        "{{ x: {}, y: {} }}",
        item.x, item.y
    )));
}
