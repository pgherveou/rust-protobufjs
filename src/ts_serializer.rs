use crate::{field::FieldRule, message::Message, namespace::Namespace, r#enum::Enum, r#type::Type};
use std::collections::HashMap;

trait Printable {
    fn print(&self) -> &str;
}

impl Printable for String {
    fn print(&self) -> &str {
        self.as_str()
    }
}

impl Printable for &str {
    fn print(&self) -> &str {
        self
    }
}

fn format_error_types<'a>(v: Vec<HTTPErrorType<'a>>) -> String {
    v.iter()
        .map(|HTTPErrorType { code, type_name }| format!("[code: {}, body: {}]", code, type_name))
        .collect::<Vec<_>>()
        .join(" | ")
}

struct HTTPErrorType<'a> {
    code: &'a str,
    type_name: &'a str,
}

struct HTTPOptions<'a> {
    path: &'a str,
    method: &'a str,
    error_types: Vec<HTTPErrorType<'a>>,
}

impl<'a> HTTPOptions<'a> {
    pub fn from(raw_options: &'a Vec<Vec<String>>) -> Option<Self> {
        let mut path = None;
        let mut method = None;
        let mut error_types = Vec::new();

        for option in raw_options {
            let option = option.iter().map(String::as_str).collect::<Vec<_>>();

            match option[..] {
                ["pgm.http.rule", mthod, pth] => {
                    path.replace(pth);
                    method.replace(mthod);
                }
                ["pgm.error.rule", "default_error_type", type_name, ..] => {
                    error_types.push(HTTPErrorType {
                        code: "number",
                        type_name,
                    });

                    for error_override in option[3..].chunks(5) {
                        match error_override {
                            ["error_override", "type", type_name, "code", code]
                            | ["error_override", "code", code, "type", type_name] => {
                                error_types.push(HTTPErrorType { code, type_name });
                            }
                            _ => {}
                        }
                    }
                }
                ["http.http_options", ".path", v] => {
                    path.replace(v);
                }
                ["http.http_options", ".method", v] => {
                    method.replace(v);
                }
                ["http.http_options", ".error_type", type_name] => {
                    error_types.push(HTTPErrorType {
                        code: "number",
                        type_name,
                    });
                }
                ["http.http_options", ".error_overrides", "code", code, "type", type_name]
                | ["http.http_options", ".error_overrides", "type", type_name, "code", code] => {
                    error_types.push(HTTPErrorType { code, type_name });
                }

                _ => {}
            }
        }

        match (path, method) {
            (Some(path), Some(method)) => Some(HTTPOptions {
                method,
                path,
                error_types,
            }),
            _ => None,
        }
    }
}

pub struct Printer {
    buffer: String,
    print_bubble_client: bool,
    print_network_client: bool,
}

impl Printer {
    /// Returns a new printer with default options
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            print_bubble_client: true,
            print_network_client: true,
        }
    }

    pub fn to_string(mut self, root: &Namespace) -> String {
        if self.print_bubble_client {
            self.println("declare module '@lyft/bubble-client' {");
            self.println("interface Router {");
            self.write_bubble_client_types(root);
            self.println("}");
            self.println("}");
        }

        if self.print_network_client {
            self.println("declare module '@lyft/network-client' {");
            self.println("interface NetworkClient {");
            self.write_network_client_types(root);
            self.println("}");
            self.println("}");
        }

        self.println("declare global {");
        self.write_namespaces(&root.nested);
        self.println("}");

        self.buffer
    }

    fn write_bubble_client_types(&mut self, ns: &Namespace) {
        for (ns_name, ns) in ns.nested.iter() {
            for service in ns.services.values() {
                for (method_name, rpc) in service.methods.iter() {
                    let req = rpc_type(rpc.request_type.borrow().as_str(), rpc.request_stream);
                    let resp = rpc_type(rpc.response_type.borrow().as_str(), rpc.response_stream);

                    self.println(
                    match HTTPOptions::from(&rpc.options) {
                        Some(HTTPOptions{path, method, error_types}) => {
                            format!("{method}(path: '{path}', handler: RouteHandler<{req}, {resp}, {code_error_tuples}>): void", 
                            method = method.to_lowercase(),
                            path = path,
                            req = req, resp = resp,
                            code_error_tuples = format_error_types(error_types)
                            )
                        }
                        None => {
                            format!("grpc(path: '/{ns_name}/{method_name}', handler: RouteHandler<{req}, {resp}, [code: number, body: string]>): void", 
                            ns_name = ns_name,
                            method_name = method_name,
                            req = req, resp = resp)
                        }
                    });
                }
            }

            self.write_bubble_client_types(ns);
        }
    }

    fn write_network_client_types(&mut self, ns: &Namespace) {
        for (ns_name, ns) in ns.nested.iter() {
            for service in ns.services.values() {
                for (method_name, rpc) in service.methods.iter() {
                    let req = rpc_type(rpc.request_type.borrow().as_str(), rpc.request_stream);
                    let resp = rpc_type(rpc.response_type.borrow().as_str(), rpc.response_stream);
                    self.println(format!("grpc(path: '/{}/{}', handler: RouteHandler<{}, {}, [code: number, body: string]>): void", ns_name, method_name, req, resp));
                }
            }

            self.write_bubble_client_types(ns);
        }
    }

    fn write_namespaces(&mut self, namespaces: &HashMap<String, Namespace>) {
        for (name, ns) in namespaces {
            self.println(format!("namespace {} {{", name));
            self.write_types(&ns.types);
            self.write_namespaces(&ns.nested);
            self.println("}");
        }
    }

    fn write_types(&mut self, types: &HashMap<String, Type>) {
        for (name, t) in types.iter() {
            match t {
                Type::Message(msg) => {
                    self.println(format!("interface {} {{", name));
                    self.write_message(msg, name);
                }
                Type::Enum(e) => {
                    self.println(format!("const enum {} {{", name));
                    self.write_enum(e);
                }
            }
            self.println("}");
        }
    }

    fn write_message(&mut self, msg: &Message, name: &String) {
        for (name, field) in msg.fields.iter() {
            let type_name = field.type_name.borrow();
            self.println(match (&field.key_type, &field.rule) {
                (Some(key), _) => format!("{}: {{ [key: {}]: {} }}", name, key, type_name),
                (None, Some(FieldRule::Repeated)) => format!("{}: Array<{}>", name, type_name),
                (None, _) => format!("{}: {}", name, type_name),
            });
        }

        if !msg.nested.is_empty() {
            self.println(format!("namespace {} {{", name));
            self.write_types(&msg.nested);
            self.println("}");
        }
    }

    fn write_enum(&mut self, e: &Enum) {
        for (name, value) in e.values.iter() {
            self.println(format!("{} = {},", name, value));
        }
    }

    fn println<T>(&mut self, value: T)
    where
        T: Printable,
    {
        self.buffer.push_str(value.print());
        self.buffer.push('\n')
    }
}

fn rpc_type(type_name: &str, is_streaming: bool) -> String {
    if is_streaming {
        format!("Observable<{}>", type_name)
    } else {
        type_name.to_string()
    }
}
