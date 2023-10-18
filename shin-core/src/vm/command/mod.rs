//! Defines the commands that can be produced by the VM and executed by the engine.

pub mod types;

use crate::format::scenario::instruction_elements::Register;
use crate::format::scenario::instructions::{BitmaskNumberArray, MessageId, NumberSpec};
use crate::format::scenario::types::U8SmallNumberList;
use crate::format::text::{StringArray, U16FixupString, U16String};
use crate::time::Ticks;
use shin_derive::Command;
// those are actually used by the generated code (it's a bit messy, i know)
#[allow(unused)]
use types::{
    AudioWaitStatus, LayerCtrlFlags, LayerId, LayerProperty, LayerPropertySmallList, LayerType,
    MaskFlags, MessageboxStyle, Pan, VLayerId, Volume,
};

#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[derive(Command, Debug)]
/// This is a fake command type, real commands are generated by the derive macro
///
/// we want each command to be a separate struct, but defining them is much easier with one enum
///
/// The derive macro will generate two versions of each command:
/// - [CompiletimeCommand] A compile time representation, which is mostly the same as this enum
/// - [RuntimeCommand] A runtime representation, which has some types replaced by their runtime equivalents (e.g. `NumberSpec` -> `i32`)
//
// TODO: describe logic with commands that return a value (it's a bit complicated and i haven't thought it through yet)
// TODO: maybe UX of the derive macro is not the best. consider using build.rs-based codegen
pub enum Command {
    #[cmd(opcode = 0x00u8)]
    EXIT {
        /// This is encoded in the instruction
        /// If it's zero then the VM shuts down
        /// If it's nonzero then the VM treats it as a NOP
        /// Maybe it's a feature that is not used for umineko?
        arg1: u8,
        /// Return value? Not sure tbh
        arg2: NumberSpec,
    },

    /// Get persistent value
    #[cmd(opcode = 0x81u8)]
    SGET {
        #[cmd(dest)]
        dest: Register,
        slot_number: NumberSpec,
    },
    /// Set persistent value
    #[cmd(opcode = 0x82u8)]
    SSET {
        slot_number: NumberSpec,
        value: NumberSpec,
    },
    /// Delay the execution for `wait_amount` ticks
    #[cmd(opcode = 0x83u8)]
    WAIT {
        /// If true - allow skipping the wait by pressing "advance" button
        #[cmd(rty = "bool")]
        allow_interrupt: u8,
        #[cmd(rty = "Ticks")]
        wait_amount: NumberSpec,
    },
    // 0x84 is unused
    /// Set messagebox style & text layout
    #[cmd(opcode = 0x85u8)]
    MSGINIT {
        #[cmd(rty = "MessageboxStyle")]
        messagebox_style: NumberSpec,
    },
    /// Show the message
    ///
    /// The text may contain various commands that are executed in parallel with the VM
    #[cmd(opcode = 0x86u8)]
    MSGSET {
        /// Message ID, used to mark whether the user has seen this message
        msg_id: MessageId,
        /// If true - do not continue execution until the message is finished
        ///
        /// If the message is not waited, [MSGWAIT](Command::MSGWAIT) can be called to synchronize with parts the message
        #[cmd(rty = "bool")]
        auto_wait: u8,
        text: U16FixupString,
    }, // TODO: this string needs a fixup (see ShinDataUtil's OpcodeDefinitions.NeedsStringFixup)
    /// Waits for message to reach the specified section
    ///
    /// -1 means wait for the message to finish fully
    #[cmd(opcode = 0x87u8)]
    MSGWAIT { section_num: NumberSpec },
    /// Signal to the message @y command
    #[cmd(opcode = 0x88u8)]
    MSGSIGNAL {},
    /// Not used in the scenario, semantics unclear
    #[cmd(opcode = 0x89u8)]
    MSGSYNC { arg1: NumberSpec, arg2: NumberSpec },
    /// Close the messagebox
    #[cmd(opcode = 0x8au8)]
    MSGCLOSE {
        /// If true - wait for the messagebox close animation to finish
        #[cmd(rty = "bool")]
        wait_for_close: u8,
    },

    /// Show a choice menu, store the selected variant in `dest`
    #[cmd(opcode = 0x8du8)]
    SELECT {
        choice_set_base: u16,
        choice_index: u16,
        #[cmd(dest)]
        dest: Register,
        choice_visibility_mask: NumberSpec,
        choice_title: U16String,
        variants: StringArray,
    },
    #[cmd(opcode = 0x8eu8)]
    WIPE {
        arg1: NumberSpec,
        arg2: NumberSpec,
        wipe_time: NumberSpec,
        params: BitmaskNumberArray,
    },
    #[cmd(opcode = 0x8fu8)]
    WIPEWAIT {},
    /// Start a BGM track
    #[cmd(opcode = 0x90u8)]
    BGMPLAY {
        /// BGM ID (stored in scenario header)
        bgm_data_id: NumberSpec,
        #[cmd(rty = "Ticks")]
        fade_in_time: NumberSpec,
        /// If true - do not restart the track when it's finished
        #[cmd(rty = "bool")]
        no_repeat: NumberSpec,
        #[cmd(rty = "Volume")]
        volume: NumberSpec,
    },
    /// Stop the current BGM track
    #[cmd(opcode = 0x91u8)]
    BGMSTOP {
        #[cmd(rty = "Ticks")]
        fade_out_time: NumberSpec,
    },
    /// Change the volume of the current BGM track
    #[cmd(opcode = 0x92u8)]
    BGMVOL {
        #[cmd(rty = "Volume")]
        volume: NumberSpec,
        #[cmd(rty = "Ticks")]
        fade_in_time: NumberSpec,
    },
    /// Wait for the BGM track to reach the specified status
    #[cmd(opcode = 0x93u8)]
    BGMWAIT {
        #[cmd(rty = "AudioWaitStatus")]
        target_status: NumberSpec,
    },
    /// Wait for BGM to reach the specified time ¿in ticks?
    #[cmd(opcode = 0x94u8)]
    BGMSYNC { sync_time: NumberSpec },
    /// Start a SE track in the specified slot
    #[cmd(opcode = 0x95u8)]
    SEPLAY {
        se_slot: NumberSpec,
        se_data_id: NumberSpec,
        #[cmd(rty = "Ticks")]
        fade_in_time: NumberSpec,
        #[cmd(rty = "bool")]
        no_repeat: NumberSpec,
        #[cmd(rty = "Volume")]
        volume: NumberSpec,
        #[cmd(rty = "Pan")]
        pan: NumberSpec,
        play_speed: NumberSpec,
    },
    /// Stop a SE track in the specified slot
    #[cmd(opcode = 0x96u8)]
    SESTOP {
        se_slot: NumberSpec,
        #[cmd(rty = "Ticks")]
        fade_out_time: NumberSpec,
    },
    /// Stop all SE tracks
    #[cmd(opcode = 0x97u8)]
    SESTOPALL {
        #[cmd(rty = "Ticks")]
        fade_out_time: NumberSpec,
    },
    /// Change the volume of a SE track in the specified slot
    #[cmd(opcode = 0x98u8)]
    SEVOL {
        se_slot: NumberSpec,
        #[cmd(rty = "Volume")]
        volume: NumberSpec,
        #[cmd(rty = "Ticks")]
        fade_in_time: NumberSpec,
    },
    /// Change the pan of a SE track in the specified slot
    #[cmd(opcode = 0x99u8)]
    SEPAN {
        se_slot: NumberSpec,
        #[cmd(rty = "Pan")]
        pan: NumberSpec,
        #[cmd(rty = "Ticks")]
        fade_in_time: NumberSpec,
    },
    /// Wait for a SE track in the specified slot to reach the specified status
    #[cmd(opcode = 0x9au8)]
    SEWAIT {
        se_slot: NumberSpec, // may have a special value of -1
        #[cmd(rty = "AudioWaitStatus")]
        target_status: NumberSpec,
    },
    /// ¿Play an SE without a slot?
    #[cmd(opcode = 0x9bu8)]
    SEONCE {
        arg1: NumberSpec,
        arg2: NumberSpec,
        arg3: NumberSpec,
        arg4: NumberSpec,
        arg5: NumberSpec,
    },
    #[cmd(opcode = 0x9cu8)]
    VOICEPLAY {
        name: U16String,
        #[cmd(rty = "Volume")]
        volume: NumberSpec,
        flags: NumberSpec,
    },
    #[cmd(opcode = 0x9du8)]
    VOICESTOP {},
    #[cmd(opcode = 0x9eu8)]
    VOICEWAIT {
        #[cmd(rty = "AudioWaitStatus")]
        target_status: NumberSpec,
    },
    /// Play a system sound effect (from `/sysse.bin`)
    #[cmd(opcode = 0x9fu8)]
    SYSSE { arg1: NumberSpec, arg2: NumberSpec },

    /// Set current save info at specified level
    /// (0 - scenario name, 1 - chapter name)
    ///
    /// It can be seen in the pause menu, in the save/load screen
    ///
    /// It is also shown temporarily in bottom left corner of the screen when changed
    #[cmd(opcode = 0xa0u8)]
    SAVEINFO {
        level: NumberSpec,
        info: U16FixupString,
    },
    /// Save the game to autosave slot
    #[cmd(opcode = 0xa1u8)]
    AUTOSAVE {},
    #[cmd(opcode = 0xa2u8)]
    EVBEGIN { arg: NumberSpec },
    #[cmd(opcode = 0xa3u8)]
    EVEND {},
    #[cmd(opcode = 0xa4u8)]
    RESUMESET {},
    #[cmd(opcode = 0xa5u8)]
    RESUME {},
    #[cmd(opcode = 0xa6u8)]
    SYSCALL { arg1: NumberSpec, arg2: NumberSpec },

    /// Give the player a trophy (only implemented on PS4?)
    #[cmd(opcode = 0xb0u8)]
    TROPHY { trophy_id: NumberSpec },
    /// Unlock a CG, BGM or MOVIE
    #[cmd(opcode = 0xb1u8)]
    UNLOCK {
        unlock_type: u8,
        unlock_indices: U8SmallNumberList,
    },

    /// Reset property values to their initial state
    #[cmd(opcode = 0xc0u8)]
    LAYERINIT {
        #[cmd(rty = "VLayerId")]
        layer_id: NumberSpec,
    },
    /// Load a user layer
    /// There are multiple layer types and they have different arguments
    #[cmd(opcode = 0xc1u8)]
    LAYERLOAD {
        #[cmd(rty = "VLayerId")]
        layer_id: NumberSpec,
        #[cmd(rty = "LayerType")]
        layer_type: NumberSpec,
        // TODO: what does this mean again?
        leave_uninitialized: NumberSpec,
        params: BitmaskNumberArray,
    },
    #[cmd(opcode = 0xc2u8)]
    LAYERUNLOAD {
        #[cmd(rty = "VLayerId")]
        layer_id: NumberSpec,
        delay_time: NumberSpec,
    },
    /// Change layer property, possibly through a transition.
    #[cmd(opcode = 0xc3u8)]
    LAYERCTRL {
        #[cmd(rty = "VLayerId")]
        layer_id: NumberSpec,
        #[cmd(rty = "LayerProperty")]
        property_id: NumberSpec,
        /// (target_value, time, flags, easing_param)
        #[cmd(rty = "(i32, Ticks, LayerCtrlFlags, i32)")]
        params: BitmaskNumberArray,
    },
    /// Wait for property transitions to finish.
    #[cmd(opcode = 0xc4u8)]
    LAYERWAIT {
        #[cmd(rty = "VLayerId")]
        layer_id: NumberSpec,
        #[cmd(rty = "LayerPropertySmallList")]
        wait_properties: U8SmallNumberList,
    },
    #[cmd(opcode = 0xc5u8)]
    LAYERSWAP { arg1: NumberSpec, arg2: NumberSpec },
    /// Select a subset of layers to perform batch operations
    /// (TODO: fact check) These can be used as layer_id = -4
    #[cmd(opcode = 0xc6u8)]
    LAYERSELECT {
        // AFAIK, those can't use the virtual layer numbers
        #[cmd(rty = "LayerId")]
        selection_start_id: NumberSpec,
        #[cmd(rty = "LayerId")]
        selection_end_id: NumberSpec,
    },
    #[cmd(opcode = 0xc7u8)]
    MOVIEWAIT {
        #[cmd(rty = "LayerId")]
        layer_id: NumberSpec,
        target_status: NumberSpec,
    },
    // 0xc8 unused
    #[cmd(opcode = 0xc9u8)]
    TRANSSET {
        arg1: NumberSpec,
        arg2: NumberSpec,
        arg3: NumberSpec,
        params: BitmaskNumberArray,
    },
    #[cmd(opcode = 0xcau8)]
    TRANSWAIT { arg: NumberSpec },
    #[cmd(opcode = 0xcbu8)]
    PAGEBACK {},
    #[cmd(opcode = 0xccu8)]
    PLANESELECT { plane_id: NumberSpec },
    #[cmd(opcode = 0xcdu8)]
    PLANECLEAR {},
    #[cmd(opcode = 0xceu8)]
    MASKLOAD {
        mask_data_id: NumberSpec,
        #[cmd(rty = "MaskFlags")]
        mask_flags: NumberSpec,
        #[cmd(rty = "bool")]
        smth_smth_transition: NumberSpec,
    },
    #[cmd(opcode = 0xcfu8)]
    MASKUNLOAD {},

    /// Unlock a character in the character screen
    #[cmd(opcode = 0xe0u8)]
    CHARS { arg1: NumberSpec, arg2: NumberSpec },
    /// Unlock a TIP in the TIPS menu
    #[cmd(opcode = 0xe1u8)]
    TIPSGET { tip_ids: U8SmallNumberList },
    /// Show a quiz??
    #[cmd(opcode = 0xe2u8)]
    QUIZ {
        #[cmd(dest)]
        dest: Register,
        arg: NumberSpec,
    },
    /// Show "Characters" menu
    #[cmd(opcode = 0xe3u8)]
    SHOWCHARS {},
    /// Show notification, like "Characters menu updated" (I think?)
    /// the argument seems to be the notification type
    #[cmd(opcode = 0xe4u8)]
    NOTIFYSET { arg: NumberSpec },

    /// Print a debug message to the console
    ///
    /// It is formatted with a printf-like syntax. Only %d seen so far.
    #[cmd(opcode = 0xffu8)]
    DEBUGOUT {
        format: U16String,
        args: U8SmallNumberList,
    },
}

/// An untyped result of a command execution. This is usually obtained by using a command token.
#[derive(Debug, Clone, Copy)]
pub enum CommandResult {
    /// No result
    None,
    /// The result is a single integer that should be written to the given memory address
    WriteMemory(Register, i32),
}

impl RuntimeCommand {
    #[inline]
    pub fn execute_dummy(self) -> Option<CommandResult> {
        Some(match self {
            RuntimeCommand::EXIT(_) => {
                // TODO: actually the logic behind this is a bit more complex
                // works for now though
                return None;
            }
            RuntimeCommand::SGET(cmd) => cmd.token.finish(0),
            RuntimeCommand::SSET(cmd) => cmd.token.finish(),
            RuntimeCommand::WAIT(cmd) => cmd.token.finish(),
            RuntimeCommand::MSGINIT(cmd) => cmd.token.finish(),
            RuntimeCommand::MSGSET(cmd) => cmd.token.finish(),
            RuntimeCommand::MSGWAIT(cmd) => cmd.token.finish(),
            RuntimeCommand::MSGSIGNAL(cmd) => cmd.token.finish(),
            RuntimeCommand::MSGSYNC(cmd) => cmd.token.finish(),
            RuntimeCommand::MSGCLOSE(cmd) => cmd.token.finish(),
            RuntimeCommand::SELECT(cmd) => cmd.token.finish(0),
            RuntimeCommand::WIPE(cmd) => cmd.token.finish(),
            RuntimeCommand::WIPEWAIT(cmd) => cmd.token.finish(),
            RuntimeCommand::BGMPLAY(cmd) => cmd.token.finish(),
            RuntimeCommand::BGMSTOP(cmd) => cmd.token.finish(),
            RuntimeCommand::BGMVOL(cmd) => cmd.token.finish(),
            RuntimeCommand::BGMWAIT(cmd) => cmd.token.finish(),
            RuntimeCommand::BGMSYNC(cmd) => cmd.token.finish(),
            RuntimeCommand::SEPLAY(cmd) => cmd.token.finish(),
            RuntimeCommand::SESTOP(cmd) => cmd.token.finish(),
            RuntimeCommand::SESTOPALL(cmd) => cmd.token.finish(),
            RuntimeCommand::SEVOL(cmd) => cmd.token.finish(),
            RuntimeCommand::SEPAN(cmd) => cmd.token.finish(),
            RuntimeCommand::SEWAIT(cmd) => cmd.token.finish(),
            RuntimeCommand::SEONCE(cmd) => cmd.token.finish(),
            RuntimeCommand::VOICEPLAY(cmd) => cmd.token.finish(),
            RuntimeCommand::VOICESTOP(cmd) => cmd.token.finish(),
            RuntimeCommand::VOICEWAIT(cmd) => cmd.token.finish(),
            RuntimeCommand::SYSSE(cmd) => cmd.token.finish(),
            RuntimeCommand::SAVEINFO(cmd) => cmd.token.finish(),
            RuntimeCommand::AUTOSAVE(cmd) => cmd.token.finish(),
            RuntimeCommand::EVBEGIN(cmd) => cmd.token.finish(),
            RuntimeCommand::EVEND(cmd) => cmd.token.finish(),
            RuntimeCommand::RESUMESET(cmd) => cmd.token.finish(),
            RuntimeCommand::RESUME(cmd) => cmd.token.finish(),
            RuntimeCommand::SYSCALL(cmd) => cmd.token.finish(),
            RuntimeCommand::TROPHY(cmd) => cmd.token.finish(),
            RuntimeCommand::UNLOCK(cmd) => cmd.token.finish(),
            RuntimeCommand::LAYERINIT(cmd) => cmd.token.finish(),
            RuntimeCommand::LAYERLOAD(cmd) => cmd.token.finish(),
            RuntimeCommand::LAYERUNLOAD(cmd) => cmd.token.finish(),
            RuntimeCommand::LAYERCTRL(cmd) => cmd.token.finish(),
            RuntimeCommand::LAYERWAIT(cmd) => cmd.token.finish(),
            RuntimeCommand::LAYERSWAP(cmd) => cmd.token.finish(),
            RuntimeCommand::LAYERSELECT(cmd) => cmd.token.finish(),
            RuntimeCommand::MOVIEWAIT(cmd) => cmd.token.finish(),
            RuntimeCommand::TRANSSET(cmd) => cmd.token.finish(),
            RuntimeCommand::TRANSWAIT(cmd) => cmd.token.finish(),
            RuntimeCommand::PAGEBACK(cmd) => cmd.token.finish(),
            RuntimeCommand::PLANESELECT(cmd) => cmd.token.finish(),
            RuntimeCommand::PLANECLEAR(cmd) => cmd.token.finish(),
            RuntimeCommand::MASKLOAD(cmd) => cmd.token.finish(),
            RuntimeCommand::MASKUNLOAD(cmd) => cmd.token.finish(),
            RuntimeCommand::CHARS(cmd) => cmd.token.finish(),
            RuntimeCommand::TIPSGET(cmd) => cmd.token.finish(),
            RuntimeCommand::QUIZ(cmd) => cmd.token.finish(0),
            RuntimeCommand::SHOWCHARS(cmd) => cmd.token.finish(),
            RuntimeCommand::NOTIFYSET(cmd) => cmd.token.finish(),
            RuntimeCommand::DEBUGOUT(cmd) => cmd.token.finish(),
        })
    }
}
