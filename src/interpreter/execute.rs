use super::environment::Environment;
use super::eval::Eval;
use super::lox_class::LoxClass;

use super::lox_function::LoxFunction;
use super::value::{CallableValue, Value};
use crate::interpreter::eval_result::{EvalError, EvalResult};
use crate::interpreter::Interpreter;
use crate::parser::expressions::{Expr, ExprCtx};
use crate::parser::statements::Stmt;
use crate::parser::{Identifier, IdentifierHandle};
use crate::runner::EloxError;
use fnv::FnvHashMap;
use qcell::{QCell, QCellOwner};
use std::rc::Rc;

pub trait Exec {
    fn exec(&self, env: &Environment, stmt: &Stmt, token: &mut QCellOwner) -> EvalResult<()>;
}

impl Exec for Interpreter {
    fn exec(&self, env: &Environment, stmt: &Stmt, token: &mut QCellOwner) -> EvalResult<()> {
        match stmt {
            Stmt::Print(ps) => {
                let val = self.eval( env, &ps.value, token)?;
                let sss = val.to_str(self, ps.pos, token)?;
                if let Err(err) = (self.host.print)(ps.pos, sss) {
                    match err {
                        EloxError::Eval(eval_err) => Err(eval_err),
                        _ => unreachable!(),
                    }
                } else {
                    Ok(())
                }
            }
            Stmt::Expr(expr_stmt) => {
                    self.eval(env, &expr_stmt.expr, token)?;
                Ok(())
            }
            Stmt::VarDecl(decl) => {
                let mut value = Value::Nil;

                if let Some(init_expr) = &decl.initializer {
                    value = self.eval( env, init_expr, token)?;
                }

                env.define(decl.identifier.name, value, token);
                Ok(())
            }
            Stmt::Block(block) => {
                let inner_env = Environment::new(Some(env), token);

                for stmt in &block.stmts {
                    self.exec( &inner_env, stmt, token)?;
                }

                Ok(())
            }
            Stmt::If(if_stmt) => {
                if (self.eval( env, &if_stmt.condition, token)?).is_truthy() {
                    self.exec( env, &if_stmt.then_branch, token)?;
                } else {
                    if let Some(else_branch) = &if_stmt.else_branch {
                        self.exec( env, else_branch, token)?;
                    }
                }

                Ok(())
            }
            Stmt::While(while_stmt) => {
                use std::ops::Deref;
                let body = (&while_stmt.body).deref();
                while (self.eval( env, &while_stmt.condition, token)?).is_truthy() {
                    self.exec( env, body, token)?;
                }

                Ok(())
            }
            Stmt::Return(ret_stmt) => {
                let value = if let Some(val) = &ret_stmt.value {
                    self.eval( env, &val, token)?
                } else {
                    Value::Nil
                };

                Err(EvalError::Return(value))
            }
            Stmt::ClassDecl(class_decl) => {
                let mut superclass = None;
                if let Some(parent_class) = &class_decl.superclass {
                    let val = self.eval(
                        env,
                        &ExprCtx::new(Expr::Var(parent_class.clone()), class_decl.pos),
                        token,
                    )?;
                    let type_ = val.type_();
                    if let Some(callable) = &val.into_callable_value() {
                        match callable {
                            CallableValue::Class(c) => {
                                superclass = Some(Rc::clone(c));
                            }
                            _ => {
                                return Err(EvalError::SuperclassMustBeAClass(
                                    class_decl.pos,
                                    type_,
                                ))
                            }
                        }
                    } else {
                        return Err(EvalError::SuperclassMustBeAClass(class_decl.pos, type_));
                    }
                }

                let mut environment = env.clone();

                environment.define(class_decl.identifier.name, Value::Nil, token);

                if let Some(parent_class) = &superclass {
                    environment = Environment::new(Some(&environment), token);
                    environment.define(
                        Identifier::super_(),
                        Value::Callable(CallableValue::Class(Rc::clone(parent_class))),
                        token,
                    );
                }

                let mut methods: FnvHashMap<IdentifierHandle, Rc<LoxFunction>> =
                    FnvHashMap::default();

                for method in &class_decl.methods {
                    let name_handle = method.name.unwrap(); // anonymous methods caught by the parser
                    let func = LoxFunction::new(
                        method.clone(),
                        environment.clone(),
                        name_handle.name == Identifier::init(),
                        method.context_less_params(self, env, token)?,
                    );
                    methods.insert(name_handle.name, Rc::new(func));
                }

                let lox_class = Rc::new(LoxClass::new(
                    class_decl.identifier.name,
                    superclass,
                    methods,
                ));
                let callable_class = Value::Callable(CallableValue::Class(lox_class));

                if let Some(_) = &class_decl.superclass {
                    environment = env.clone();
                }

                Environment::assign(&environment, 0, class_decl.identifier.name, callable_class.clone(), token);

                Ok(())
            }
        }
    }
}
