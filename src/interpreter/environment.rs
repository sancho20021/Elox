extern crate fnv;

use super::lox_array::create_elox_array_class;
use super::lox_callable::LoxCallable;
use super::natives::Clock;
use super::value::{CallableValue, Value};
use crate::parser::{Identifier, IdentifierHandle, IdentifierHandlesGenerator};
use fnv::FnvHashMap;
use qcell::{QCell, QCellOwner};
use std::rc::Rc;

#[derive(Clone)]
pub struct Environment {
    // we need multiple mutable refs to the parent scope in multiple same-level scopes -> Rc<RefCell>
    pub current: Rc<QCell<InnerEnv>>,
}

// #[derive(Debug)]
pub struct InnerEnv {
    pub values: FnvHashMap<IdentifierHandle, Value>,
    pub parent: Option<Rc<QCell<Environment>>>,
}

impl Environment {
    pub fn new(parent: Option<Rc<QCell<Environment>>>, token: &QCellOwner) -> Environment {
        let current = InnerEnv {
            values: FnvHashMap::default(),
            parent: if let Some(p) = parent {
                Some(p) // inexpensive clone
            } else {
                None
            },
        };

        Environment {
            current: Rc::new(QCell::new(token.id(), current)),
        }
    }

    pub fn with_natives(
        parent: Option<Rc<QCell<Environment>>>,
        identifiers: &mut IdentifierHandlesGenerator,
        token: &mut QCellOwner,
    ) -> Rc<QCell<Self>> {
        let env = Rc::new(QCell::new(token.id(), Environment::new(parent, token)));
        Self::register_natives(&Rc::clone(&env), identifiers, token);

        env
    }

    fn register_natives(sself: &Rc<QCell<Self>>, identifiers: &mut IdentifierHandlesGenerator, token: &mut QCellOwner) {
        Self::define(
            sself,
            identifiers.by_name("clock"),
            Value::Callable(CallableValue::Native(Rc::new(Clock))),
            token
        );

        Self::define(
            sself,
            Identifier::array(),
            Value::Callable(CallableValue::Class(Rc::new(create_elox_array_class(
                sself.ro(token),
                identifiers,
                token,
            )))),
            token
        );
    }

    pub fn define(sself: &Rc<QCell<Self>>, identifier: IdentifierHandle, value: Value, token: &mut QCellOwner) {
        let current = Rc::clone(&sself.ro(token).current);
        current.rw(token).values.insert(identifier, value);
    }

    pub fn define_no_rc(&self, identifier: IdentifierHandle, value: Value, token: &mut QCellOwner) {
        let current = &self.current;
        current.rw(token).values.insert(identifier, value);
    }

    pub fn get(&self, depth: usize, identifier: IdentifierHandle, token: &QCellOwner) -> Option<Value> {
        let current = self.current.ro(token);

        if depth == 0 {
            if let Some(value) = current.values.get(&identifier) {
                return Some(value.clone()); // inexpensive clone
            }
        } else {
            if let Some(parent) = &current.parent {
                return parent.ro(token).get(depth - 1, identifier, token);
            }
        }

        None
    }

    // pub fn assign(&self, depth: usize, identifier: IdentifierHandle, value: Value, token: &mut QCellOwner) -> bool {
    //     if depth == 0 {
    //         self.current.rw(token).values.insert(identifier, value);
    //         return true;
    //     } else if let Some(parent) = &self.current.ro(token).parent {
    //         return parent.assign(depth - 1, identifier, value, token);
    //     }

    //     false
    // }

    pub fn assign_qcell(sself: &Rc<QCell<Self>>, depth: usize, identifier: IdentifierHandle, value: Value, token: &mut QCellOwner) -> bool {
        if depth == 0 {
            let current = Rc::clone(&sself.ro(token).current);
            current.rw(token).values.insert(identifier, value);
            return true;
        } else if let Some(parent) = &sself.ro(token).current.ro(token).parent {
            let parent = Rc::clone(parent);
            return Self::assign_qcell(&parent, depth - 1, identifier, value, token);
        }

        false
    }
}
