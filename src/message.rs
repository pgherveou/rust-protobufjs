#[derive(Debug)]
pub struct Message {
    pub name: String,
    oneofs: Vec<Oneof>,
    fields: Vec<Field>,
    nested: Vec<Message>,
    enums: Vec<Enum>,
}

#[derive(Debug)]
pub struct EnumTuple(String, u32);

#[derive(Debug)]
pub struct Enum {
    name: String,
    values: Vec<EnumTuple>,
}

impl Enum {
    pub fn new(name: String) -> Self {
        Self {
            name,
            values: Vec::new(),
        }
    }

    pub fn add(&mut self, key: String, value: u32) {
        self.values.push(EnumTuple(key, value));
    }
}

#[derive(Debug)]
pub struct Oneof {
    name: String,
    field_names: Vec<String>,
}

impl Oneof {
    pub fn new(name: String) -> Self {
        Self {
            name,
            field_names: Vec::new(),
        }
    }

    pub fn add_field_name(&mut self, name: &str) {
        self.field_names.push(name.to_string());
    }
}

#[derive(Debug)]
pub struct Field {
    pub id: u32,
    pub name: String,
    pub key_type: Option<String>,
    pub type_name: String,
    pub repeated: bool,
}

impl Field {
    pub fn new(
        id: u32,
        name: String,
        type_name: String,
        repeated: bool,
        key_type: Option<String>,
    ) -> Field {
        Self {
            id,
            name,
            type_name,
            repeated,
            key_type,
        }
    }
}

impl Message {
    pub fn new(name: String) -> Message {
        Self {
            name,
            fields: Vec::new(),
            nested: Vec::new(),
            oneofs: Vec::new(),
            enums: Vec::new(),
        }
    }

    pub fn add_oneof(&mut self, oneof: Oneof) {
        self.oneofs.push(oneof);
    }

    pub fn add_enum(&mut self, e: Enum) {
        self.enums.push(e);
    }

    pub fn add_field(&mut self, field: Field) {
        self.fields.push(field);
    }

    pub fn add_nested(&mut self, message: Message) {
        self.nested.push(message);
    }
}
