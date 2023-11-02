use std::marker::PhantomData;

use shin_core::format::scenario::instructions::Instruction;

use super::{from_instr_args::FromInstrArgs, instr_lowerer::InstrLowerFn};
use crate::compile::{
    hir,
    hir::lower::from_hir::{FromHirBlockCtx, FromHirCollectors},
};

pub trait Router {
    fn handle_instr(
        &self,
        collectors: &mut FromHirCollectors,
        ctx: &FromHirBlockCtx,
        instr_name: &str,
        instr: hir::InstructionId,
    ) -> Option<Instruction>;
}

pub struct SentinelRouter;

impl Router for SentinelRouter {
    #[inline]
    fn handle_instr(
        &self,
        collectors: &mut FromHirCollectors,
        _ctx: &FromHirBlockCtx,
        instr_name: &str,
        instr: hir::InstructionId,
    ) -> Option<Instruction> {
        collectors.emit_diagnostic(
            instr.into(),
            format!("Unrecognized instruction: `{}`", instr_name),
        );
        None
    }
}

pub struct ConsRouter<Dummy, Args: FromInstrArgs, LowerFn: InstrLowerFn<Dummy, Args>, Tail: Router>
{
    name: &'static str,
    lower_fn: LowerFn,
    tail: Tail,
    phantom: PhantomData<(Dummy, Args)>,
}

impl<Dummy, Args: FromInstrArgs, LowerFn: InstrLowerFn<Dummy, Args>, Tail: Router> Router
    for ConsRouter<Dummy, Args, LowerFn, Tail>
{
    #[inline]
    fn handle_instr(
        &self,
        collectors: &mut FromHirCollectors,
        ctx: &FromHirBlockCtx,
        instr_name: &str,
        instr: hir::InstructionId,
    ) -> Option<Instruction> {
        if instr_name == self.name {
            let args = Args::from_instr_args(collectors, ctx, instr)?;
            self.lower_fn.lower_instr(collectors, ctx, instr_name, args)
        } else {
            self.tail.handle_instr(collectors, ctx, instr_name, instr)
        }
    }
}

pub struct RouterBuilder<S: Router = SentinelRouter> {
    router: S,
}

impl RouterBuilder<SentinelRouter> {
    #[inline]
    pub fn new() -> Self {
        Self {
            router: SentinelRouter,
        }
    }
}

impl<S: Router> RouterBuilder<S> {
    #[inline]
    pub fn add<Dummy, Args: FromInstrArgs, LowerFn: InstrLowerFn<Dummy, Args>>(
        self,
        name: &'static str,
        lower_fn: LowerFn,
    ) -> RouterBuilder<ConsRouter<Dummy, Args, LowerFn, S>> {
        RouterBuilder {
            router: ConsRouter {
                name,
                lower_fn,
                tail: self.router,
                phantom: PhantomData,
            },
        }
    }

    #[inline]
    pub fn build(self) -> S {
        self.router
    }
}
