use super::prelude::*;

impl StartableCommand for command::runtime::EVBEGIN {
    type StateInfo = ();
    fn apply_state(&self, _state: &mut VmState) {
        warn!("TODO: EVBEGIN state: {:?}", self);
    }

    fn start(
        self,
        _context: &UpdateContext,
        _scenario: &Arc<Scenario>,
        _vm_state: &VmState,
        _state_info: (),
        _adv_state: &mut AdvState,
    ) -> CommandStartResult {
        warn!("TODO: EVBEGIN: {:?}", self);
        self.token.finish().into()
    }
}
