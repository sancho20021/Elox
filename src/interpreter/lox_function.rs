use qcell::{QCell, QCellOwner};

use super::eval_result::{EvalError, EvalResult};
use super::execute::Exec;
use super::lox_callable::LoxCallable;
use super::lox_instance::{LoxInstance, NativesMap};
use super::Environment;
use super::Interpreter;
use crate::parser::expressions::ContextLessFuncParam;
use crate::runner::interp;

use super::Value;
use crate::parser::expressions::FuncExpr;
use crate::parser::{Identifier, IdentifierHandle, IdentifierNames};
use crate::scanner::token::Position;
use std::ops::Deref;
use std::rc::Rc;

pub type NativeFunction =
    Fn(&LoxFunction, &Interpreter, &Environment, Vec<Value>, Position) -> EvalResult<Value>;

pub type NativeMethod = Fn(
    &LoxInstance,
    &mut NativesMap,
    &LoxFunction,
    &Rc<QCell<Interpreter>>,
    &Rc<QCell<Environment>>,
    Vec<Value>,
    Position,
    &mut QCellOwner,
) -> EvalResult<Value>;

#[derive(Clone)]
pub enum Func {
    Expr(Rc<FuncExpr>),
    Native(Rc<NativeFunction>),
    NativeMethod(Rc<NativeMethod>),
}

pub type LoxFunctionParams = Option<Rc<Vec<ContextLessFuncParam>>>;

fn has_rest_param(params: &LoxFunctionParams) -> bool {
    if let Some(params) = params {
        params.iter().any(|p| p.is_rest())
    } else {
        false
    }
}

pub struct LoxFunction {
    pub func: Func,
    pub name: Option<IdentifierHandle>,
    pub env: Rc<QCell<Environment>>,
    pub is_initializer: bool,
    has_rest_param: bool,
    params: LoxFunctionParams,
}

impl LoxFunction {
    pub fn new(
        func: FuncExpr,
        env: Rc<QCell<Environment>>,
        is_initializer: bool,
        params: LoxFunctionParams,
    ) -> LoxFunction {
        LoxFunction {
            func: Func::Expr(Rc::new(func)),
            env,
            is_initializer,
            has_rest_param: has_rest_param(&params),
            params,
            name: None,
        }
    }

    pub fn new_native(
        func: Rc<NativeFunction>,
        env: Rc<QCell<Environment>>,
        is_initializer: bool,
        params: LoxFunctionParams,
        name: IdentifierHandle,
    ) -> LoxFunction {
        LoxFunction {
            func: Func::Native(func),
            env,
            is_initializer,
            has_rest_param: has_rest_param(&params),
            params,
            name: Some(name),
        }
    }

    pub fn new_native_method(
        method: Rc<NativeMethod>,
        env: Rc<QCell<Environment>>,
        is_initializer: bool,
        params: LoxFunctionParams,
        name: IdentifierHandle,
    ) -> LoxFunction {
        LoxFunction {
            func: Func::NativeMethod(method),
            env,
            is_initializer,
            has_rest_param: has_rest_param(&params),
            params,
            name: Some(name),
        }
    }

    pub fn bind(&self, instance: &LoxInstance, token: &mut QCellOwner) -> LoxFunction {
        let new_env = Rc::new(QCell::new(token.id(), Environment::new(Some(Rc::clone(&self.env)), token)));
        Environment::define(&new_env, Identifier::this(), Value::Instance(instance.clone()), token);

        match &self.func {
            Func::Expr(func_expr) => LoxFunction::new(
                func_expr.deref().clone(),
                new_env,
                self.is_initializer,
                self.params.clone(),
            ),
            Func::Native(callable) => LoxFunction::new_native(
                Rc::clone(&callable),
                new_env,
                self.is_initializer,
                self.params.clone(),
                self.name.unwrap(),
            ),
            Func::NativeMethod(callable) => LoxFunction::new_native_method(
                Rc::clone(&callable),
                new_env,
                self.is_initializer,
                self.params.clone(),
                self.name.unwrap(),
            ),
        }
    }

    pub fn pos(&self) -> Option<Position> {
        match &self.func {
            Func::Expr(func) => Some(func.pos),
            _ => None, // Native functions don't have positions
        }
    }
}

impl LoxCallable for LoxFunction {
    fn call(
        &self,
        interpreter: &Rc<QCell<Interpreter>>,
        env: &Rc<QCell<Environment>>,
        args: Vec<Value>,
        call_pos: Position,
        token: &mut QCellOwner,
    ) -> EvalResult<Value> {
        match &self.func {
            Func::Native(callable) => (callable)(&self, interpreter.ro(token), env.ro(token), args, call_pos),
            Func::NativeMethod(method) => {
                let this = self
                    .env
                    .ro(token)
                    .get(0, Identifier::this(), token)
                    .expect("Could not find 'this'")
                    .into_instance()
                    .unwrap();
                if let Some(ref mut natives) = this.instance.borrow_mut().natives {
                    return (method)(&this, natives, &self, interpreter, env, args, call_pos, token);
                }
                panic!("Could not fetch natives from native method");
            }
            Func::Expr(func) => {
                let func_env = Environment::new(Some(Rc::clone(&self.env)), token);

                if let Some(params) = &func.params {
                    for (index, param) in params.iter().enumerate() {
                        func_env.define_no_rc(param.identifier().name, args[index].clone(), token);
                    }
                }

                let init_return = if self.is_initializer {
                    if let Some(val) = self.env.ro(token).get(0, Identifier::this(), token) {
                        Some(val)
                    } else {
                        None
                    }
                } else {
                    None
                };

                let func_env = Rc::new(QCell::new(token.id(), func_env));
                for stmt in &func.body {
                    match Interpreter::exec(interpreter, &func_env, stmt, token) {
                        Err(EvalError::Return(val)) => {
                            if let Some(this) = init_return {
                                return Ok(this);
                            }

                            return Ok(val);
                        }
                        Err(e) => return Err(e),
                        Ok(_) => {}
                    };
                }

                if let Some(this) = init_return {
                    return Ok(this);
                }

                Ok(Value::Nil)
            }
        }
    }

    fn params(&self) -> LoxFunctionParams {
        self.params.clone()
    }

    fn name(&self, names: &Rc<IdentifierNames>) -> String {
        let handle = match &self.func {
            Func::Expr(func) => {
                if let Some(handle) = func.name {
                    handle.name
                } else {
                    Identifier::anonymous()
                }
            }
            _ => self.name.unwrap(),
        };

        names[handle].clone()
    }

    fn has_rest_param(&self) -> bool {
        self.has_rest_param
    }
}

impl std::fmt::Debug for LoxFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let name = match &self.func {
            Func::Native(_) => "native",
            Func::NativeMethod(_) => "native method",
            Func::Expr(func) => {
                if let Some(_) = &func.name {
                    "func"
                } else {
                    "anonymous"
                }
            }
        };

        write!(f, "<function {}>", name)
    }
}
