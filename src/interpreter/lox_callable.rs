use qcell::QCell;
use qcell::QCellOwner;

use super::lox_class::LoxClass;
use super::lox_function::LoxFunction;
use super::lox_function::LoxFunctionParams;
use super::natives::Clock;
use super::value::Value;
use super::Environment;
use super::EvalResult;
use super::Interpreter;
use crate::parser::IdentifierNames;
use crate::scanner::token::Position;
use std::rc::Rc;

pub trait LoxCallable {
    fn call(
        &self,
        interpreter: &Rc<QCell<Interpreter>>,
        env: &Rc<QCell<Environment>>,
        args: Vec<Value>,
        call_pos: Position,
        token: &mut QCellOwner,
    ) -> EvalResult<Value>;

    fn params(&self) -> LoxFunctionParams;

    fn name(&self, names: &Rc<IdentifierNames>) -> String;

    fn has_rest_param(&self) -> bool;
}
