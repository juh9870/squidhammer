use crate::graph::execution::partial::PartialGraphExecutionContext;
use crate::graph::node::SnarlNode;
use crate::registry::ETypesRegistry;
use egui_snarl::{InPinId, NodeId, OutPinId, Snarl};
use itertools::Itertools;

#[derive(derive_more::Debug)]
pub enum SnarlCommand {
    /// Connects an output pin to an input pin, marking the input pin's node as dirty
    Connect { from: OutPinId, to: InPinId },
    /// Disconnects an output pin from an input pin, marking the input pin's node as dirty
    Disconnect { from: OutPinId, to: InPinId },
    /// Disconnects all connections coming from the pin, marking the output pins' node as dirty
    DropOutputs { from: OutPinId },
    /// Deletes a node, running disconnection logic and marking connected nodes as dirty
    DeleteNode { node: NodeId },
    /// Disconnects and reconnects an output pin to an input pin, running the connected nodes' logic
    ///
    /// The most likely use case for this is when a node's output pin type
    /// changes, this allows to propagate the change and clean up the now invalid connections
    ReconnectOutput { id: OutPinId },
    /// Disconnects and reconnects all connection to the input pin, running the connected nodes' logic
    ///
    /// The most likely use case for this is when a node's input pin type
    /// changes, this allows to propagate the change and clean up the now invalid connections
    ReconnectInput { id: InPinId },
    /// Marks a node as dirty
    MarkDirty { node: NodeId },
    /// Removes the inline input value of the pin
    DeletePinValue { pin: InPinId },
    /// Connects an output pin to an input pin, marking the input pin's node as dirty
    ///
    /// This command bypasses node logic, and should only be used by the node
    /// itself, after the connect logic has been done
    ConnectRaw { from: OutPinId, to: InPinId },
    /// Disconnects an output pin from an input pin, marking the input pin's node as dirty
    ///
    /// This command bypasses node logic, and should only be used by the node
    /// itself, after the disconnect logic has been done
    DisconnectRaw { from: OutPinId, to: InPinId },
    /// Disconnects all connections to the pin, marking the input pin's node as dirty
    ///
    /// This command bypasses node logic, and should only be used by the node
    /// itself, after the disconnect logic has been done
    DropInputsRaw { to: InPinId },
    // Is this ever a good idea?
    // /// Disconnects all connections coming from the pin, marking the output pins' node as dirty
    // ///
    // /// This command bypasses node logic, and should only be used by the node
    // /// itself, after the disconnect logic has been done
    // DropOutputsRaw { from: OutPinId },
    /// Changes all connections to the input pin to point at the new pin
    ///
    /// This command bypasses node logic, and should only be used with caution
    InputMovedRaw { from: InPinId, to: InPinId },
    /// Changes all connections from the output pin to point at the new pin
    ///
    /// This command bypasses node logic, and should only be used with caution
    OutputMovedRaw { from: OutPinId, to: OutPinId },
    /// Runs a custom callback on the graph and execution context
    ///
    /// Don't forget to mark nodes as dirty if needed, either in the callback or as a separate command
    Custom {
        #[debug("fn()")]
        cb: CustomCommand,
    },
}

type CustomCommand = Box<
    dyn FnOnce(
        &mut PartialGraphExecutionContext,
        &mut Snarl<SnarlNode>,
        &ETypesRegistry,
    ) -> miette::Result<()>,
>;

impl SnarlCommand {
    pub fn execute(
        self,
        ctx: &mut PartialGraphExecutionContext,
        snarl: &mut Snarl<SnarlNode>,
        registry: &ETypesRegistry,
        commands: &mut SnarlCommands,
    ) -> miette::Result<()> {
        match self {
            SnarlCommand::DisconnectRaw { from, to } => {
                snarl.disconnect(from, to);
                ctx.mark_dirty(snarl, to.node);
            }
            SnarlCommand::ConnectRaw { from, to } => {
                snarl.connect(from, to);
                ctx.mark_dirty(snarl, to.node);
            }
            SnarlCommand::InputMovedRaw { from, to } => {
                if let Some(value) = ctx.inputs.remove(&from) {
                    ctx.inputs.insert(to, value);
                }
                let pin = snarl.in_pin(from);
                snarl.drop_inputs(from);
                for remote in pin.remotes {
                    snarl.connect(remote, to);
                }
                if from.node != to.node {
                    ctx.mark_dirty(snarl, from.node);
                }
                ctx.mark_dirty(snarl, to.node);
            }
            SnarlCommand::OutputMovedRaw { from, to } => {
                let pin = snarl.out_pin(from);
                snarl.drop_outputs(from);
                for remote in pin.remotes {
                    snarl.connect(to, remote);
                    ctx.mark_dirty(snarl, remote.node);
                }
            }
            SnarlCommand::MarkDirty { node } => {
                ctx.mark_dirty(snarl, node);
            }
            SnarlCommand::Custom { cb } => {
                cb(ctx, snarl, registry)?;
            }
            SnarlCommand::ReconnectOutput { id } => {
                for pin in snarl.out_pin(id).remotes {
                    SnarlCommand::Disconnect { from: id, to: pin }
                        .execute(ctx, snarl, registry, commands)?;
                    SnarlCommand::Connect { from: id, to: pin }
                        .execute(ctx, snarl, registry, commands)?;
                }
            }
            SnarlCommand::ReconnectInput { id } => {
                for pin in snarl.in_pin(id).remotes {
                    SnarlCommand::Disconnect { from: pin, to: id }
                        .execute(ctx, snarl, registry, commands)?;
                    SnarlCommand::Connect { from: pin, to: id }
                        .execute(ctx, snarl, registry, commands)?;
                }
            }
            SnarlCommand::Connect { from, to } => {
                let from = snarl.out_pin(from);
                let to = snarl.in_pin(to);
                ctx.connect(&from, &to, snarl, commands)?;
            }
            SnarlCommand::Disconnect { from, to } => {
                let from = snarl.out_pin(from);
                let to = snarl.in_pin(to);
                ctx.disconnect(&from, &to, snarl, commands)?;
            }
            SnarlCommand::DropInputsRaw { to } => {
                snarl.drop_inputs(to);
                ctx.mark_dirty(snarl, to.node);
            }
            // SnarlCommand::DropOutputsRaw { from } => {
            //     for pin in snarl.out_pin(from).remotes {
            //         ctx.mark_dirty(snarl, pin.node);
            //     }
            //     snarl.drop_outputs(from);
            // }
            SnarlCommand::DeletePinValue { pin } => {
                ctx.inputs.remove(&pin);
            }
            SnarlCommand::DropOutputs { from } => {
                for pin in snarl.out_pin(from).remotes {
                    SnarlCommand::Disconnect { from, to: pin }
                        .execute(ctx, snarl, registry, commands)?;
                }
            }
            SnarlCommand::DeleteNode { node } => {
                // Disconnect all outputs
                for (out_pin, in_pin) in snarl.wires().filter(|w| w.0.node == node).collect_vec() {
                    SnarlCommand::Disconnect {
                        from: out_pin,
                        to: in_pin,
                    }
                    .execute(ctx, snarl, registry, commands)?;
                }
                // Disconnect all inputs
                for (out_pin, in_pin) in snarl.wires().filter(|w| w.1.node == node).collect_vec() {
                    ctx.inputs.remove(&in_pin);
                    SnarlCommand::Disconnect {
                        from: out_pin,
                        to: in_pin,
                    }
                    .execute(ctx, snarl, registry, commands)?;
                }
                snarl.remove_node(node);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct SnarlCommands {
    commands: Vec<SnarlCommand>,
}

impl SnarlCommands {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, command: SnarlCommand) {
        self.commands.push(command);
    }

    pub fn execute(
        &mut self,
        ctx: &mut PartialGraphExecutionContext,
        snarl: &mut Snarl<SnarlNode>,
    ) -> miette::Result<()> {
        let mut iter = 0;
        while !self.commands.is_empty() {
            iter += 1;
            if iter > 1000 {
                panic!("Node commands formed an infinite loop");
            }
            let mut commands = std::mem::take(&mut self.commands);
            for command in commands.drain(..) {
                command.execute(ctx, snarl, ctx.registry, self)?;
            }
            if self.commands.is_empty() {
                self.commands = commands;
            }
        }
        Ok(())
    }
}
