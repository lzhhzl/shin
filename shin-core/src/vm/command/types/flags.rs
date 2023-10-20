use crate::format::scenario::instruction_elements::{FromNumber, NumberSpec, UntypedNumberSpec};
use crate::vm::{FromVmCtx, VmCtx};
use bitflags::bitflags;
use proc_bitfield::bitfield;

bitfield! {
    /// Flags that can be used in [LAYERCTRL](super::super::runtime::LAYERCTRL) command
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct LayerCtrlFlags(pub i32) : Debug {
        /// Which easing function to use (see [Easing](crate::time::Easing))
        pub easing: i32 @ 0..6,
        pub scale_time: bool @ 6,
        /// If true - the target value is relative to the current value
        pub delta: bool @ 7,
        pub ff_to_current: bool @ 8,
        pub ff_to_target: bool @ 9,
        pub unused_1: i32 @ 10..12,
        pub prohibit_fast_forward: bool @ 12,
        pub unused_2: i32 @ 13..16,
        pub ignore_wait: bool @ 16,
        pub unused_3: i32 @ 17..32,
    }
}

impl FromNumber for LayerCtrlFlags {
    fn from_number(number: i32) -> Self {
        Self(number)
    }
}

// TODO: remove this when BNA is refactored
impl FromVmCtx<UntypedNumberSpec> for LayerCtrlFlags {
    fn from_vm_ctx(ctx: &VmCtx, input: UntypedNumberSpec) -> Self {
        Self(ctx.get_number(NumberSpec::new(input)))
    }
}

bitflags! {
    /// Flags that can be used in [MASKLOAD](super::super::runtime::MASKLOAD) command
    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
    pub struct MaskFlags: i32 {
        const FLIP_X = 0x0001;
        const FLIP_Y = 0x0002;
        const UNK_4 = 0x0004;
        const SCALE = 0x0010;
    }
}

impl FromNumber for MaskFlags {
    fn from_number(number: i32) -> Self {
        MaskFlags::from_bits(number).expect("Invalid MaskFlags")
    }
}

bitflags! {
    /// Represents a status of a playing audio that can be awaited on
    ///
    /// Used in [BGMWAIT](super::super::runtime::BGMWAIT), [SEWAIT](super::super::runtime::SEWAIT) and [VOICEWAIT](super::super::runtime::VOICEWAIT) commands
    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
    pub struct AudioWaitStatus: i32 {
        const PLAYING = 1;
        const STOPPED = 2;
        const VOLUME_TWEENER_IDLE = 4;
        const PANNING_TWEENER_IDLE = 8;
        const PLAY_SPEED_TWEENER_IDLE = 16;
    }
}

impl FromNumber for AudioWaitStatus {
    fn from_number(number: i32) -> Self {
        AudioWaitStatus::from_bits(number).expect("Invalid AudioWaitStatus")
    }
}
