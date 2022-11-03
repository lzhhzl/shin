use crate::vm::Vm;
use bevy::prelude::*;
use shin_core::vm::command;
use shin_core::vm::command::CommandResult;

#[derive(Component)]
pub struct SESTOPALL;

impl super::Command<command::runtime::SESTOPALL> for SESTOPALL {
    type Result = CommandResult;

    fn start(command: command::runtime::SESTOPALL, vm: &mut Vm) -> Self::Result {
        warn!("TODO: SESTOPALL: {:?}", command);
        command.token.finish()
    }
}
