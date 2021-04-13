use std::collections::HashMap;

#[derive(Debug)]
pub struct Message {
    pub name: String,
    fields: HashMap<String, Field>,
    nested: Vec<Message>,
}

#[derive(Debug)]
pub struct Field {
    pub id: u32,
    pub type_name: String,
    pub repeated: bool,
}

impl Field {
    pub fn new(id: u32, type_name: String, repeated: bool) -> Field {
        Self {
            id,
            type_name,
            repeated,
        }
    }
}

impl Message {
    pub fn new(name: String) -> Message {
        Self {
            name,
            fields: HashMap::new(),
            nested: Vec::new(),
        }
    }

    pub fn add_field(&mut self, name: String, field: Field) {
        self.fields.insert(name, field);
    }

    pub fn add_nested(&mut self, message: Message) {
        self.nested.push(message);
    }
}
