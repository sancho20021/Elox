use qcell::{QCell, QCellOwner};

use super::environment::Environment;
use super::eval_result::{EvalError, EvalResult};
use super::lox_array::new_elox_array;
use super::lox_callable::LoxCallable;
use super::lox_function::LoxFunction;
use super::value::{CallableValue, Value};
use crate::interpreter::Interpreter;
use crate::parser::expressions::ContextLessFuncParam;
use crate::parser::expressions::{
    BinaryOperator, BinaryOperatorCtx, Expr, ExprCtx, Literal, LogicalOperator, UnaryOperator,
};
use crate::parser::Identifier;
use std::ops::Deref;
use std::rc::Rc;

pub trait Eval {
    fn eval(sself: &Rc<QCell<Self>>, env: &Rc<QCell<Environment>>, expr: &ExprCtx, token: &mut QCellOwner) -> EvalResult<Value>;
}

impl Eval for Interpreter {
    fn eval(sself: &Rc<QCell<Self>>, env: &Rc<QCell<Environment>>, expr_ctx: &ExprCtx, token: &mut QCellOwner) -> EvalResult<Value> {
        match &expr_ctx.expr {
            Expr::Literal(literal) => match literal {
                Literal::Number(ref n) => Ok(Value::Number(*n)),
                Literal::String(s) => Ok(Value::String(s.clone())),
                Literal::Nil => Ok(Value::Nil),
                Literal::Boolean(b) => Ok(Value::Boolean(*b)),
            },
            Expr::Grouping(sub_expr) => Self::eval(sself, env, &sub_expr.deref().expression, token),
            Expr::Unary(sub_expr) => {
                let expr = sub_expr.deref();
                let val = Self::eval(sself, env, &expr.right, token)?;
                match expr.operator {
                    UnaryOperator::Minus => {
                        if let Value::Number(nb) = val {
                            Ok(Value::Number(-nb))
                        } else {
                            Err(EvalError::UnexpectedUnaryOperatorOperand(
                                expr.right.pos,
                                UnaryOperator::Minus,
                                val.type_(),
                            ))
                        }
                    }
                    UnaryOperator::Bang => Ok(Value::Boolean(!val.is_truthy())),
                }
            }
            Expr::Binary(bin_expr) => {
                let expr = bin_expr.deref();
                let a = Self::eval(sself, env, &expr.left, token)?;
                let b = Self::eval(sself, env, &expr.right, token)?;

                let op_ctx = &expr.operator;

                match op_ctx.op {
                    BinaryOperator::Minus => {
                        arithmetic_op(op_ctx, &a, &b, |a, b| Value::Number(a - b))
                    }
                    BinaryOperator::Plus => match (&a, &b) {
                        (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a + b)),
                        (_, _) => Ok(Value::String(format!(
                            "{}{}",
                            a.to_str(sself, expr.left.pos, token)?,
                            b.to_str(sself, expr.right.pos, token)?
                        ))),
                    },
                    BinaryOperator::Slash => {
                        arithmetic_op(op_ctx, &a, &b, |a, b| Value::Number(a / b))
                    }
                    BinaryOperator::Star => {
                        arithmetic_op(op_ctx, &a, &b, |a, b| Value::Number(a * b))
                    }
                    BinaryOperator::Percent => {
                        arithmetic_op(op_ctx, &a, &b, |a, b| Value::Number(a % b))
                    }

                    BinaryOperator::Greater => {
                        arithmetic_op(op_ctx, &a, &b, |a, b| Value::Boolean(a > b))
                    }
                    BinaryOperator::GreaterEqual => {
                        arithmetic_op(op_ctx, &a, &b, |a, b| Value::Boolean(a >= b))
                    }
                    BinaryOperator::Less => {
                        arithmetic_op(op_ctx, &a, &b, |a, b| Value::Boolean(a < b))
                    }
                    BinaryOperator::LessEqual => {
                        arithmetic_op(op_ctx, &a, &b, |a, b| Value::Boolean(a <= b))
                    }
                    BinaryOperator::EqualEqual => Ok(Value::Boolean(a == b)),
                    BinaryOperator::BangEqual => Ok(Value::Boolean(a != b)),
                }
            }

            Expr::Var(var_expr) => {
                if let Some(value) = sself.ro(token).lookup_variable(env.ro(token), &var_expr.identifier, token) {
                    return Ok(value);
                }

                Err(EvalError::UndefinedVariable(
                    expr_ctx.pos,
                    sself.ro(token).name(var_expr.identifier.name),
                ))
            }
            Expr::Assign(expr) => {
                let assign_expr = expr.deref();
                let value = Self::eval(sself, env, &assign_expr.expr, token)?;

                if Self::assign_variable(sself,env, &expr.identifier, value.clone(), token) {
                    return Ok(value);
                }

                Err(EvalError::UndefinedVariable(
                    expr_ctx.pos,
                    sself.ro(token).name(assign_expr.identifier.name),
                ))
            }
            Expr::Logical(expr) => {
                let left = Self::eval(sself, env, &expr.left, token)?;
                let truthy = left.is_truthy();

                match &expr.operator {
                    LogicalOperator::Or => {
                        if truthy {
                            return Ok(left);
                        }
                    }
                    LogicalOperator::And => {
                        if !truthy {
                            return Ok(left);
                        }
                    }
                }

                Self::eval(sself, env, &expr.right, token)
            }
            Expr::Call(call_expr) => {
                let callee = Self::eval(sself, env, &call_expr.callee, token)?;

                let mut args = Vec::with_capacity(call_expr.args.len());

                for arg in &call_expr.args {
                    args.push(Self::eval(sself, env, arg, token)?);
                }

                match callee {
                    Value::Callable(callable_value) => {
                        let callable = callable_value.into_callable();
                        let has_rest_param = callable.has_rest_param();
                        match callable.params() {
                            Some(params) => {
                                // default values
                                if params.len() != args.len() || has_rest_param {
                                    for param in params.iter().skip(args.len()) {
                                        use ContextLessFuncParam::*;
                                        match param {
                                            DefaultValued(_, val) => {
                                                args.push(val.clone());
                                            }
                                            _ => break,
                                        };
                                    }

                                    // if rest: push the params into a native Array
                                    if has_rest_param && args.len() >= params.len() {
                                        // a rest parameter is always the last one
                                        let rest_params = args.split_off(params.len() - 1);
                                        args.push(new_elox_array(rest_params, sself.ro(token), token));
                                    } else if has_rest_param && args.len() == params.len() - 1 {
                                        args.push(new_elox_array(vec![], sself.ro(token), token));
                                    }

                                    let min_args = params
                                        .iter()
                                        .filter(|p| p.is_required())
                                        .collect::<Vec<_>>()
                                        .len();

                                    let max_args = if has_rest_param {
                                        usize::max_value()
                                    } else {
                                        params.len()
                                    };

                                    if min_args == max_args && !has_rest_param {
                                        return Err(EvalError::WrongNumberOfArgs(
                                            expr_ctx.pos,
                                            min_args,
                                            args.len(),
                                            callable.name(&sself.ro(token).names),
                                        ));
                                    } else if args.len() < min_args || args.len() > max_args {
                                        return Err(EvalError::WrongNumberOfArgsBetween(
                                            expr_ctx.pos,
                                            min_args,
                                            max_args,
                                            args.len(),
                                            callable.name(&sself.ro(token).names),
                                        ));
                                    }
                                }
                            }
                            None => {
                                if !args.is_empty() {
                                    return Err(EvalError::WrongNumberOfArgs(
                                        expr_ctx.pos,
                                        0,
                                        args.len(),
                                        callable.name(&sself.ro(token).names),
                                    ));
                                }
                            }
                        };

                        return Ok(callable.call(sself, env, args, expr_ctx.pos, token)?);
                    }
                    _ => return Err(EvalError::ValueNotCallable(expr_ctx.pos, callee.type_())),
                }
            }
            Expr::Func(func_expr) => {
                let func = LoxFunction::new(
                    func_expr.clone(),
                    Rc::new(QCell::new(token.id(), env.ro(token).clone())), // inexpensive clone
                    false,
                    func_expr.context_less_params(sself, env, token)?,
                );

                let f = Value::Callable(CallableValue::Function(Rc::new(func)));

                // if not anonymous
                if let Some(identifier) = func_expr.name {
                    Environment::define(env, identifier.name, f.clone(), token);
                }

                Ok(f)
            }
            Expr::Get(get_expr) => {
                let val = Self::eval(sself, env, &get_expr.object, token)?;

                if let Some(instance) = val.into_instance() {
                    if let Some(prop_val) = instance.get(get_expr.property.name, token) {
                        return Ok(prop_val);
                    } else {
                        return Err(EvalError::UndefinedProperty(
                            expr_ctx.pos,
                            sself.ro(token).name(get_expr.property.name),
                        ));
                    }
                }

                Err(EvalError::OnlyInstancesHaveProperties(
                    expr_ctx.pos,
                    Self::eval(sself, env, &get_expr.object, token).unwrap().type_(),
                ))
            }
            Expr::Set(set_expr) => {
                let obj = Self::eval(sself, env, &set_expr.object, token)?;

                if let Some(instance) = &obj.into_instance() {
                    let val = Self::eval(sself, env, &set_expr.value, token)?;
                    instance.set(set_expr.property.name, &val);
                    return Ok(val);
                }

                Err(EvalError::OnlyInstancesHaveProperties(
                    expr_ctx.pos,
                    Self::eval(sself, env, &set_expr.object, token).unwrap().type_(),
                ))
            }
            Expr::This(this_expr) => {
                if let Some(this) = sself.ro(token).lookup_variable(env.ro(token), &this_expr.identifier, token) {
                    return Ok(this);
                }

                Ok(Value::Nil)
            }
            Expr::Super(super_expr) => {
                if let Some(&depth) = sself.ro(token).resolver.depth(super_expr.identifier.use_handle) {
                    if let Some(superclass) = env.ro(token).get(depth, Identifier::super_(), token) {
                        if let Some(Value::Instance(instance)) =
                            env.ro(token).get(depth - 1, Identifier::this(), token)
                        {
                            if let Some(CallableValue::Class(parent)) =
                                superclass.into_callable_value()
                            {
                                if let Some(method) = parent.find_method(super_expr.method.name) {
                                    return Ok(Value::Callable(CallableValue::Function(Rc::new(
                                        method.bind(&instance, token),
                                    ))));
                                } else {
                                    return Err(EvalError::UndefinedProperty(
                                        expr_ctx.pos,
                                        sself.ro(token).name(super_expr.method.name),
                                    ));
                                }
                            }
                        }
                    }
                }

                Ok(Value::Nil)
            }
            Expr::ArrayDeclExpr(array_decl) => {
                let values = array_decl
                    .values
                    .iter()
                    .map(|val| Self::eval(sself, env, &val, token))
                    .collect::<EvalResult<Vec<_>>>()?;
                Ok(new_elox_array(values, sself.ro(token), token))
            }
        }
    }
}

#[inline]
fn arithmetic_op<F>(
    op_ctx: &BinaryOperatorCtx,
    a: &Value,
    b: &Value,
    operation: F,
) -> EvalResult<Value>
where
    F: Fn(&f64, &f64) -> Value,
{
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => Ok(operation(a, b)),
        _ => Err(EvalError::UnexpectedBinaryOperatorOperands(
            op_ctx.pos.clone(),
            op_ctx.op.clone(),
            a.type_(),
            b.type_(),
        )),
    }
}
