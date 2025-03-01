use either::Either;
use rustc_hash::FxHashMap;

use crate::{
    compile::{
        def_map::Name,
        diagnostics::{Diagnostic, Span},
        hir,
        hir::{
            lower::{LowerError, LowerResult},
            Expr,
        },
        make_diagnostic,
    },
    syntax::{ast, ast::UnaryOp},
};

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct ConstexprValue(i32);

impl ConstexprValue {
    pub fn constant(value: i32) -> Self {
        Self(value)
    }

    pub fn value(self) -> i32 {
        self.0
    }
}

impl std::fmt::Debug for ConstexprValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value())
    }
}

pub enum ConstexprContextValue {
    Value(LowerResult<ConstexprValue>, Option<Span>),
    // This only exists to make the constexpr evaluator know that the value exists, but is of wrong type
    // we can't really meaningfully use a block reference in a constexpr expression, so there's no value associated with it
    Block(Span),
}

// NOTE: potentially we might want to add support for evaluating expressions in instruction arguments
// however, it would require some design work to make sure it is sane (for example, we might want to do const fold instead of evaluating, think of `$v1 + 2 * 8`)
// thus, for now, constexpr evaluation only happens in value alias context
// (these different contexts will require some adapters)
pub type ConstexprContext = FxHashMap<Name, ConstexprContextValue>;

fn type_mismatch(
    location: Either<hir::ExprId, Span>,
    expected: &str,
    found: &str,
) -> Diagnostic<Either<hir::ExprId, Span>> {
    make_diagnostic!(
        location,
        "Type mismatch: expected {}, found {}",
        expected,
        found
    )
}

struct EvaluateContext<'a> {
    context: &'a ConstexprContext,
    diagnostics: &'a mut Vec<Diagnostic<Either<hir::ExprId, Span>>>,
    block: &'a hir::HirBlockBody,
    // TODO: hir source map
}

impl EvaluateContext<'_> {
    fn error(
        &mut self,
        diagnostic: Diagnostic<Either<hir::ExprId, Span>>,
    ) -> LowerResult<ConstexprValue> {
        self.diagnostics.push(diagnostic);
        Err(LowerError)
    }
}

fn evaluate(ctx: &mut EvaluateContext, expr: hir::ExprId) -> LowerResult<ConstexprValue> {
    match ctx.block.exprs[expr] {
        Expr::Missing => Err(LowerError),
        Expr::Literal(hir::Literal::IntNumber(value)) => Ok(ConstexprValue::constant(value)),
        Expr::Literal(hir::Literal::RationalNumber(value)) => {
            Ok(ConstexprValue::constant(value.into_raw()))
        }
        Expr::Literal(hir::Literal::String(_)) => {
            ctx.error(type_mismatch(Either::Left(expr), "int or float", "string"))
        }
        Expr::NameRef(ref name) => match ctx.context[name] {
            ConstexprContextValue::Value(ref value, _) => *value,
            ConstexprContextValue::Block(location) => ctx.error(
                type_mismatch(Either::Left(expr), "int or float", "code reference")
                    .with_additional_label(
                        "Code reference defined at".to_string(),
                        Either::Right(location),
                    ),
            ),
        },
        Expr::RegisterRef(_) => ctx.error(make_diagnostic!(
            Either::Left(expr),
            "Registers cannot be used in const context"
        )),
        Expr::Array(_) => ctx.error(type_mismatch(Either::Left(expr), "int or float", "array")),
        Expr::Mapping(_) => ctx.error(type_mismatch(Either::Left(expr), "int or float", "mapping")),
        Expr::UnaryOp { expr: val, op } => {
            let ConstexprValue(val) = evaluate(ctx, val)?;

            let result = match op {
                UnaryOp::Negate => val.checked_neg(),
                UnaryOp::LogigalNot => Some(if val == 0 { 1 } else { 0 }),
                UnaryOp::BitwiseNot => Some(!val),
            };

            result
                .map(ConstexprValue::constant)
                .ok_or(())
                .or_else(|()| {
                    ctx.error(make_diagnostic!(
                        Either::Left(expr),
                        "Overflow in constant expression"
                    ))
                })
        }
        Expr::BinaryOp { lhs, rhs, op } => {
            let lhs = evaluate(ctx, lhs);
            let rhs = evaluate(ctx, rhs);
            let (Ok(ConstexprValue(lhs)), Ok(ConstexprValue(rhs)), Some(op)) = (lhs, rhs, op)
            else {
                return Err(LowerError);
            };

            let result = match op {
                ast::BinaryOp::Add => lhs.checked_add(rhs),
                ast::BinaryOp::Subtract => lhs.checked_sub(rhs),
                ast::BinaryOp::Multiply => lhs.checked_mul(rhs),
                ast::BinaryOp::Divide => {
                    if rhs == 0 {
                        return ctx.error(make_diagnostic!(Either::Left(expr), "Division by zero"));
                    }
                    lhs.checked_div(rhs)
                }
                op => todo!("constexpr evaluation of {:?}", op),
            };

            match result {
                Some(result) => Ok(ConstexprValue::constant(result)),
                None => ctx.error(make_diagnostic!(
                    Either::Left(expr),
                    "Overflow in constant expression"
                )),
            }
        }
        Expr::Call { .. } => {
            todo!()
        }
    }
}

pub fn constexpr_evaluate(
    context: &ConstexprContext,
    block: &hir::HirBlockBody,
    expr: hir::ExprId,
) -> (
    LowerResult<ConstexprValue>,
    Vec<Diagnostic<Either<hir::ExprId, Span>>>,
) {
    let mut diagnostics = Vec::new();

    let mut ctx = EvaluateContext {
        context,
        diagnostics: &mut diagnostics,
        block,
    };

    let value = evaluate(&mut ctx, expr);

    (value, diagnostics)
}
