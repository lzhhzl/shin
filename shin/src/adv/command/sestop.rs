use shin_core::time::Tween;

use super::prelude::*;

impl StartableCommand for command::runtime::SESTOP {
    type StateInfo = ();
    fn apply_state(&self, state: &mut VmState) {
        state.audio.se[self.se_slot as usize] = None;
    }

    fn start(
        self,
        _context: &mut UpdateContext,
        _scenario: &Arc<Scenario>,
        _vm_state: &VmState,
        _state_info: (),
        adv_state: &mut AdvState,
    ) -> CommandStartResult {
        adv_state
            .se_player
            .stop(self.se_slot, Tween::linear(self.fade_out_time));

        self.token.finish().into()
    }
}
