use crate::vm::Vm;
use bevy::prelude::*;
use shin_core::vm::command;
use shin_core::vm::command::CommandResult;

#[derive(Component)]
pub struct MSGCLOSE;

impl super::Command<command::runtime::MSGCLOSE> for MSGCLOSE {
    type Result = CommandResult;

    fn start(command: command::runtime::MSGCLOSE, _vm: &mut Vm) -> Self::Result {
        assert_eq!(command.arg, 0);
        command.token.finish()
    }
}
