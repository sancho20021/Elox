pub mod environment;

pub mod eval;
pub mod eval_result;
mod execute;
pub mod host;
pub mod lexical_scope;
pub mod lox_array;
pub mod lox_callable;
mod lox_class;
pub mod lox_function;
mod lox_instance;
mod natives;
pub mod value;

use crate::parser::{
    statements::Stmt, IdentifierHandle, IdentifierNames, IdentifierUse,
};
use environment::Environment;
use eval_result::EvalResult;
use execute::Exec;
use host::Host;
use qcell::{QCell, QCellOwner};
use std::rc::Rc;
use value::Value;
use lexical_scope::Resolver;

pub struct Interpreter {
    global: Environment,
    resolver: Resolver,
    host: Rc<Host>,
    names: Rc<IdentifierNames>,
}

impl Interpreter {
    pub fn new(env: Environment, host: &Rc<Host>, names: &Rc<IdentifierNames>, resolver: Resolver) -> Interpreter {
        Interpreter {
            global: env,
            resolver,
            host: Rc::clone(host),
            names: Rc::clone(names),
        }
    }

    pub fn interpret(&mut self, stmts: &[Stmt], token: &mut QCellOwner) -> EvalResult<()> {
        for stmt in stmts {
            self.exec(&self.global, stmt, token)?;
        }

        Ok(())
    }

    pub fn name(&self, handle: IdentifierHandle) -> String {
        self.names[handle].clone()
    }

    pub fn names(&self) -> Rc<IdentifierNames> {
        Rc::clone(&self.names)
    }

    pub fn lookup_variable(&self, env: &Environment, identifier: &IdentifierUse, token: &QCellOwner) -> Option<Value> {
        let res = if let Some(&depth) = self.resolver.depth(identifier.use_handle) {
            env.get(depth, identifier.name, token)
        } else {
            self.global.get(0, identifier.name, token)
        };

        res
    }

    pub fn lookup_global(&self, name: IdentifierHandle, token: &QCellOwner) -> Option<Value> {
        self.global.get(0, name, token)
    }

    pub fn assign_variable(
        &self,
        env: &Environment,
        identifier: &IdentifierUse,
        value: Value,
        token: &mut QCellOwner,
    ) -> bool {
        if let Some(&depth) = self.resolver.depth(identifier.use_handle) {
            env.assign(depth, identifier.name, value, token)
        } else {
            self.global.assign(0, identifier.name, value, token)
        }
    }
}
