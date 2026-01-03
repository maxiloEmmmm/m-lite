use std::{cell::RefCell, rc::Rc};

pub struct Focus {
    parent: String,
    _self: String,
    current: Rc<RefCell<String>>,
}

impl Focus {
    pub fn root() -> Focus {
        Focus {
            parent: String::default(),
            _self: String::default(),
            current: Rc::new(RefCell::new(String::default())),
        }
    }

    pub fn sub(&self, key: &str) -> Focus {
        Focus {
            parent: self._self.to_owned(),
            _self: key.to_owned(),
            current: self.current.clone(),
        }
    }

    pub fn is_me(&self) -> bool {
        self._self.eq(self.current.borrow().as_str())
    }

    pub fn set(&self, key: &str) {
        *self.current.borrow_mut() = key.to_owned();
    }

    pub fn back(&self) {
        if !self.is_me() {
            return;
        }
        *self.current.borrow_mut() = self.parent.to_owned();
    }
}
