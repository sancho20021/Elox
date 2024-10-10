use crate::interpreter::value::Value;
use crate::parser::expressions::{BinaryOperator, UnaryOperator};
use crate::scanner::scanner_result::ErrorPosition;
use crate::scanner::token::Position;
use std::fmt;

pub type EvalResult<T> = Result<T, EvalError>;

pub enum EvalError {
    UnexpectedUnaryOperatorOperand(Position, UnaryOperator, String),
    UnexpectedBinaryOperatorOperands(Position, BinaryOperator, String, String),
    UndefinedVariable(Position, String),
    ValueNotCallable(Position, String),
    WrongNumberOfArgs(Position, usize, usize, String),
    WrongNumberOfArgsBetween(Position, usize, usize, usize, String),
    CouldNotGetTime(Position),
    OnlyInstancesHaveProperties(Position, String),
    UndefinedProperty(Position, String),
    SuperclassMustBeAClass(Position, String),
    ToStringMethodMustReturnAString(Position, String, String),
    ArrayIndexOutOfBounds(Position, usize, usize),
    StackOverflow(Position, usize),
    Return(Value),
}

impl fmt::Debug for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedUnaryOperatorOperand(arg0, arg1, arg2) => f.debug_tuple("UnexpectedUnaryOperatorOperand").field(arg0).field(arg1).field(arg2).finish(),
            Self::UnexpectedBinaryOperatorOperands(arg0, arg1, arg2, arg3) => f.debug_tuple("UnexpectedBinaryOperatorOperands").field(arg0).field(arg1).field(arg2).field(arg3).finish(),
            Self::UndefinedVariable(arg0, arg1) => f.debug_tuple("UndefinedVariable").field(arg0).field(arg1).finish(),
            Self::ValueNotCallable(arg0, arg1) => f.debug_tuple("ValueNotCallable").field(arg0).field(arg1).finish(),
            Self::WrongNumberOfArgs(arg0, arg1, arg2, arg3) => f.debug_tuple("WrongNumberOfArgs").field(arg0).field(arg1).field(arg2).field(arg3).finish(),
            Self::WrongNumberOfArgsBetween(arg0, arg1, arg2, arg3, arg4) => f.debug_tuple("WrongNumberOfArgsBetween").field(arg0).field(arg1).field(arg2).field(arg3).field(arg4).finish(),
            Self::CouldNotGetTime(arg0) => f.debug_tuple("CouldNotGetTime").field(arg0).finish(),
            Self::OnlyInstancesHaveProperties(arg0, arg1) => f.debug_tuple("OnlyInstancesHaveProperties").field(arg0).field(arg1).finish(),
            Self::UndefinedProperty(arg0, arg1) => f.debug_tuple("UndefinedProperty").field(arg0).field(arg1).finish(),
            Self::SuperclassMustBeAClass(arg0, arg1) => f.debug_tuple("SuperclassMustBeAClass").field(arg0).field(arg1).finish(),
            Self::ToStringMethodMustReturnAString(arg0, arg1, arg2) => f.debug_tuple("ToStringMethodMustReturnAString").field(arg0).field(arg1).field(arg2).finish(),
            Self::ArrayIndexOutOfBounds(arg0, arg1, arg2) => f.debug_tuple("ArrayIndexOutOfBounds").field(arg0).field(arg1).field(arg2).finish(),
            Self::StackOverflow(arg0, arg1) => f.debug_tuple("StackOverflow").field(arg0).field(arg1).finish(),
            Self::Return(arg0) => todo!("Cannot implement debug for QCell"),
        }
    }
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EvalError::UnexpectedUnaryOperatorOperand(_, op, type_) => write!(
                f,
                "Unexpected operand type for operator: '{}' found '{}', expected a number",
                op, type_,
            ),
            EvalError::UndefinedVariable(_, id) => write!(f, "Undefined variable: '{}'", id),
            EvalError::UnexpectedBinaryOperatorOperands(_, op, a, b) => write!(
                f,
                "Unexpected operand types for operator: '{}', found '{}' and '{}'",
                op, a, b
            ),
            EvalError::ValueNotCallable(_, typ) => {
                write!(f, "Value of type '{}' is not callable", typ)
            }
            EvalError::WrongNumberOfArgs(_, expected, got, name) => {
                let s = if *expected != 1 { "s" } else { "" };
                write!(
                    f,
                    "'{}' expected {} argument{}, got {}",
                    name, expected, s, got
                )
            }
            EvalError::WrongNumberOfArgsBetween(_, min, max, got, name) => {
                let max = if max == &usize::max_value() {
                    "infinity".into()
                } else {
                    format!("{}", max)
                };
                write!(
                    f,
                    "'{}' expected between {} and {} arguments, got {}",
                    name, min, max, got
                )
            }
            EvalError::CouldNotGetTime(_) => write!(f, "Could not get time"),
            EvalError::Return(_) => unreachable!(),
            EvalError::OnlyInstancesHaveProperties(_, typ) => {
                write!(f, "Only instances have properties, found: '{}'", typ)
            }
            EvalError::UndefinedProperty(_, id) => write!(f, "Undefined property: '{}'", id),
            EvalError::SuperclassMustBeAClass(_, typ) => {
                write!(f, "Superclass must be a class, found: '{}'", typ)
            }
            EvalError::ToStringMethodMustReturnAString(_, class_name, return_type) => write!(
                f,
                "#str trait method must return a string, got '{}' in class '{}'",
                return_type, class_name
            ),
            EvalError::ArrayIndexOutOfBounds(_, idx, len) => write!(
                f,
                "Index out of bounds: tried to access value at index {} on an array of length {}",
                idx, len
            ),
            EvalError::StackOverflow(_, max) => write!(f, "Stack overflox: max frames = {}", max),
        }
    }
}

impl ErrorPosition for EvalError {
    fn position(&self) -> &Position {
        use EvalError::*;
        match self {
            UnexpectedUnaryOperatorOperand(pos, _, _)
            | UnexpectedBinaryOperatorOperands(pos, _, _, _)
            | UndefinedVariable(pos, _)
            | ValueNotCallable(pos, _)
            | WrongNumberOfArgs(pos, _, _, _)
            | WrongNumberOfArgsBetween(pos, _, _, _, _)
            | OnlyInstancesHaveProperties(pos, _)
            | UndefinedProperty(pos, _)
            | SuperclassMustBeAClass(pos, _)
            | ToStringMethodMustReturnAString(pos, _, _)
            | ArrayIndexOutOfBounds(pos, _, _)
            | StackOverflow(pos, _)
            | CouldNotGetTime(pos) => pos,
            Return(_) => unreachable!(),
        }
    }
}
