use super::prelude::*;

impl StartableCommand for command::runtime::MSGCLOSE {
    type StateInfo = ();
    fn apply_state(&self, state: &mut VmState) {
        state.messagebox_state.messagebox_shown = false;
        state.messagebox_state.text = None;
    }

    fn start(
        self,
        _context: &mut UpdateContext,
        _scenario: &Arc<Scenario>,
        _vm_state: &VmState,
        _state_info: (),
        adv_state: &mut AdvState,
    ) -> CommandStartResult {
        assert!(!self.wait_for_close);

        adv_state.root_layer_group.message_layer_mut().close(true);

        self.token.finish().into()
    }
}
