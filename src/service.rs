#[derive(Debug)]
pub struct MethodDefinition {
    name: String,
    stream: bool,
}

impl MethodDefinition {
    pub fn new(name: String, stream: bool) -> Self {
        Self { name, stream }
    }
}

#[derive(Debug)]
pub struct Rpc {
    name: String,
    request: MethodDefinition,
    response: MethodDefinition,
}

impl Rpc {
    pub fn new(name: String, request: MethodDefinition, response: MethodDefinition) -> Self {
        Self {
            name,
            request,
            response,
        }
    }
}

#[derive(Debug)]
pub struct Service {
    pub name: String,
    pub methods: Vec<Rpc>,
}

impl Service {
    pub fn new(name: String) -> Service {
        Self {
            name,
            methods: Vec::new(),
        }
    }

    pub fn add_rpc(&mut self, rpc: Rpc) {
        self.methods.push(rpc);
    }
}
