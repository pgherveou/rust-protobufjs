use indoc::formatdoc;

use crate::{
    field::FieldRule,
    http_options::{HTTPErrorType, HTTPOptions},
    message::Message,
    metadata::Metadata,
    namespace::Namespace,
    r#enum::Enum,
    r#type::Type,
};
use std::collections::BTreeMap;

fn format_error_types(v: Vec<HTTPErrorType<'_>>) -> String {
    v.iter()
        .map(|e| e.as_string())
        .collect::<Vec<_>>()
        .join(" | ")
}

#[derive(Default)]
pub struct Printer {
    buffer: String,
    root_url: String,
    pub print_bubble_client: bool,
    pub print_network_client: bool,
}

impl Printer {
    pub fn into_string(mut self, root: &Namespace) -> String {
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
            for (name, service) in ns.services.iter() {
                self.print_comment(&service.md, name);

                for (method_name, rpc) in service.methods.iter() {
                    self.print_comment(&rpc.md, method_name);
                    let req = rpc_type(rpc.request_type.borrow().as_str(), rpc.request_stream);
                    let resp = rpc_type(rpc.response_type.borrow().as_str(), rpc.response_stream);

                    self.println(
                    match HTTPOptions::from(&rpc.md.options) {
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
            for (name, service) in ns.services.iter() {
                self.print_comment(&service.md, name);

                for (method_name, rpc) in service.methods.iter() {
                    self.print_comment(&service.md, method_name);
                    let req = rpc_type(rpc.request_type.borrow().as_str(), rpc.request_stream);
                    let resp = rpc_type(rpc.response_type.borrow().as_str(), rpc.response_stream);

                    self.println(
                        match HTTPOptions::from(&rpc.md.options) {
                            Some(HTTPOptions{path, method, ..}) => {
                                // post(path: '/foo'): HTTPResource<pb.lyft.hello.SayHelloRequest, pb.lyft.hello.SayHelloResponse>
                                format!("{method}(path: '{path}'): HTTPResource<{req}, {resp}>)", 
                                method = method.to_lowercase(),
                                path = path,
                                req = req, resp = resp,
                                )
                            }
                            None => {
                                format!("grpc(path: '/{}/{}', handler: RouteHandler<{}, {}, [code: number, body: string]>): void",                                
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

    fn write_namespaces(&mut self, namespaces: &BTreeMap<String, Namespace>) {
        for (name, ns) in namespaces {
            self.println(format!("namespace {} {{", name));
            self.write_types(ns.types.iter());
            self.write_namespaces(&ns.nested);
            self.println("}");
        }
    }

    fn write_types<'a>(&mut self, types: impl Iterator<Item = (&'a String, &'a Type)>) {
        for (name, t) in types {
            match t {
                Type::Message(msg) => {
                    self.print_comment(&msg.md, name);
                    self.println(format!("interface {} {{", name));
                    self.write_message(msg, name);
                }
                Type::Enum(e) => {
                    self.print_comment(&e.md, name);
                    self.println(format!("const enum {} {{", name));
                    self.write_enum(e);
                }
            }
            self.println("}");
        }
    }

    fn write_message(&mut self, msg: &Message, name: &str) {
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
            self.write_types(msg.nested.iter());
            self.println("}");
        }
    }

    fn write_enum(&mut self, e: &Enum) {
        for (name, value) in e.values.iter() {
            self.println(format!("{} = {},", name, value));
        }
    }

    fn println<T: AsRef<str>>(&mut self, value: T) {
        self.buffer.push_str(value.as_ref());
        self.buffer.push('\n')
    }

    fn print_comment(&mut self, md: &Metadata, default_text: &str) {
        let text = comment_text(md, default_text).split('\n');
        let text = text.collect::<Vec<_>>().join("\n * ");

        let v = formatdoc! {"
        /**
         * {text}
         * @link {url}{path}#{line}
         */
        ",
        text = text,
        url = self.root_url,
        path = md.file_path.to_str().unwrap(),
        line = md.line,
        };

        self.println(v);
    }
}

fn comment_text<'a>(md: &'a Metadata, default: &'a str) -> &'a str {
    md.comment
        .as_ref()
        .map(|cmt| cmt.text.as_str())
        .unwrap_or(default)
}

fn rpc_type(type_name: &str, is_streaming: bool) -> String {
    if is_streaming {
        format!("Observable<{}>", type_name)
    } else {
        type_name.to_string()
    }
}
