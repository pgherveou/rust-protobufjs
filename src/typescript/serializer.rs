use super::constants::TYPE_MAPPING;
use crate::{
    field::FieldRule, http_options::HTTPOptions, message::Message, metadata::Metadata,
    namespace::Namespace, r#enum::Enum, r#type::Type, service::Rpc, typescript::constants::*,
};
use convert_case::{Case, Casing};
use std::{
    borrow::Cow,
    collections::{BTreeMap, HashSet},
};

/// PrintOptions let us configure How we want to print a Proto tree into a Typescript definition file
pub struct PrintConfig {
    pub root_url: String,
    pub print_bubble_client: bool,
    pub print_network_client: bool,
}

pub struct Printer<'a> {
    buffer: String,
    config: &'a PrintConfig,
    includes: HashSet<&'static str>,
    indent: usize,
}

impl<'a> Printer<'a> {
    /// Create a new printer
    pub fn new(config: &'a PrintConfig) -> Self {
        Self {
            buffer: String::new(),
            includes: HashSet::new(),
            config,
            indent: 0,
        }
    }

    /// Create a Typescript definition file
    pub fn into_string(mut self, root: &'a Namespace) -> String {
        let mut network_client_printer = self.printer_with_config(4);
        let mut bubble_client_printer = self.printer_with_config(4);
        let mut types_printer = self.printer_with_config(2);
        let mut includes: HashSet<&'static str> = HashSet::new();

        // write messages typescript definitions
        types_printer.write_namespaces(&root.nested);

        // write services definitions
        write_services(root, &mut |ns, method_name, rpc| {
            network_client_printer.write_network_client_rpc(ns, method_name, rpc);
            bubble_client_printer.write_bubble_client_rpc(ns, method_name, rpc);
        });

        // add network client import or throw away
        if network_client_printer.config.print_network_client
            && !network_client_printer.buffer.is_empty()
        {
            includes.insert(NETWORK_CLIENT_IMPORT);
        } else {
            network_client_printer.buffer.clear()
        }

        // add bubbke client import or throw away
        if bubble_client_printer.config.print_bubble_client
            && !bubble_client_printer.buffer.is_empty()
        {
            includes.insert(BUBBLE_CLIENT_IMPORT);
        } else {
            network_client_printer.buffer.clear()
        }

        // gather includes
        for printer in [
            &bubble_client_printer,
            &network_client_printer,
            &types_printer,
        ] {
            includes.extend(&printer.includes)
        }

        // print imports from includes
        std::array::IntoIter::new([
            OBSERVABLE_IMPORT,
            BUBBLE_CLIENT_IMPORT,
            NETWORK_CLIENT_IMPORT,
        ])
        .filter(|import| includes.contains(import))
        .for_each(|import| self.println(import));

        // print @lyft/bubble-client definitions
        if !bubble_client_printer.buffer.is_empty() {
            self.println_and_indent("declare module '@lyft/bubble-client' {");
            self.println_and_indent("interface Router {");
            self.append(bubble_client_printer);
            self.outdent_and_println("}");
            self.outdent_and_println("}");
        }

        // print @lyft/network-client definitions into package_printer
        if !network_client_printer.buffer.is_empty() {
            self.println_and_indent("declare module '@lyft/network-client' {");
            self.println_and_indent("interface NetworkClient {");
            self.append(network_client_printer);
            self.outdent_and_println("}");
            self.outdent_and_println("}");
        }

        self.println("declare global {");

        // print global types from includes
        std::array::IntoIter::new([&LONG_LIKE_TYPE, &ANY_TYPE, &EMPTY])
            .filter(|val| includes.contains(*val))
            .for_each(|val| self.println(val));

        self.add_blank_line();
        self.append(types_printer);
        self.println("}");

        self.buffer
    }

    /// Write @lyft/bubble-client typescript definitions
    fn write_bubble_client_rpc(&mut self, ns: &'a Namespace, method_name: &'a str, rpc: &'a Rpc) {
        self.print_comment(&rpc.md, true);
        let req = rpc.request_type.borrow();
        let req = self.rpc_type(req.as_str(), rpc.request_stream);

        let resp = rpc.response_type.borrow();
        let resp = self.rpc_type(resp.as_str(), rpc.response_stream);

        self.println(
                    match HTTPOptions::from(&rpc.md.options) {
                        Some(HTTPOptions{path, method, error_types}) => {
                            // TODO handle empty error_types
                            let code_error_tuples = error_types.iter()
                                .map(|e| e.as_string())
                                .collect::<Vec<_>>()
                                .join(" | ");

                            format!("{method}(path: '{path}', handler: RouteHandler<{req}, {resp}, {code_error_tuples}>): void", 
                            method = method.to_lowercase(),
                            path = path,
                            req = req, resp = resp,
                            code_error_tuples = code_error_tuples
                            )
                        },
                        None => {
                            format!("grpc(path: '/{ns_name}/{method_name}', handler: RouteHandler<{req}, {resp}, [code: number, body: string]>): void", 
                            ns_name = ns.path.join("."),
                            method_name = method_name,
                            req = req, resp = resp)
                        }
                    });
    }

    /// Write @lyft/network-client typescript definitions
    fn write_network_client_rpc(&mut self, ns: &'a Namespace, method_name: &'a str, rpc: &'a Rpc) {
        let req = rpc.request_type.borrow();
        let req = self.rpc_type(req.as_str(), rpc.request_stream);

        let resp = rpc.response_type.borrow();
        let resp = self.rpc_type(resp.as_str(), rpc.response_stream);

        self.print_comment(&rpc.md, true);
        self.println(
            match HTTPOptions::from(&rpc.md.options) {
                Some(HTTPOptions{path, method, ..}) => {
                    format!("{method}(path: '{path}'): HTTPResource<{req}, {resp}>", 
                    method = method.to_lowercase(),
                    path = path,
                    req = req, resp = resp,
                    )
                }
                None => {
                    format!("grpc(path: '/{}/{}', handler: GRPCResource<{}, {}, [code: number, body: string]>): void",                                
                    ns_name = ns.path.join("."),
                    method_name = method_name,
                    req = req, resp = resp)
                }
            });
    }

    /// Write namespace typescript definitions
    fn write_namespaces(&mut self, namespaces: &'a BTreeMap<String, Namespace>) {
        for (name, ns) in namespaces {
            self.println_and_indent(format!("namespace {} {{", name));
            self.write_types(ns.types.iter());
            self.write_namespaces(&ns.nested);
            self.outdent_and_println("}");
        }
    }

    /// Write Type (Message or Enum) typescript definitions
    fn write_types(&mut self, types: impl Iterator<Item = (&'a String, &'a Type)>) {
        for (name, t) in types {
            match t {
                Type::Message(msg) => {
                    self.print_comment(&msg.md, true);
                    self.write_message(name, msg);
                }
                Type::Enum(e) => {
                    self.print_comment(&e.md, true);
                    self.println_and_indent(format!("const enum {} {{", name));
                    self.write_enum(e);
                    self.outdent_and_println("}");
                }
            }
        }
    }

    /// Write a Proto message typescript definitions
    fn write_message(&mut self, msg_name: &'a str, msg: &'a Message) {
        let mut printer = self.printer_with_config(self.indent + 2);
        let mut generic_constraints = Vec::new();

        for (name, field) in msg.fields.iter() {
            let type_name = field.type_name.borrow();

            let type_name = match type_name.as_str() {
                ".google.protobuf.Any" => {
                    self.includes.insert(ANY_TYPE);
                    let generic_name = name.to_case(Case::Pascal);
                    let type_name = format!("AnyType<{}>", generic_name);
                    generic_constraints.push(format!("{} = unknown", generic_name));
                    Cow::Owned(type_name)
                }
                name => self.get_type(name).into(),
            };

            printer.print_comment(&field.md, false);
            printer.println(match (&field.key_type, &field.rule) {
                (Some(key), _) => {
                    format!("{}?: {{ [key: {}]: {} }}", name, key, type_name)
                }
                (None, Some(FieldRule::Repeated)) => {
                    format!("{}?: Array<{}>", name, type_name)
                }
                (None, _) => format!("{}?: {}", name, type_name),
            });
        }

        match generic_constraints.len() {
            0 => match msg.fields.len() {
                0 => {
                    self.includes.insert(EMPTY);
                    self.println(format!("interface {} extends Empty {{", msg_name))
                }
                _ => self.println(format!("interface {} {{", msg_name)),
            },
            _ => self.println(format!(
                "interface {}<{}> {{",
                msg_name,
                generic_constraints.join(",")
            )),
        }

        for (name, oneof) in msg.oneofs.iter() {
            printer.print_comment(&oneof.md, false);
            printer.println(format!(
                "{}?: Extract<keyof {}, {}>",
                name,
                msg_name,
                oneof
                    .values
                    .iter()
                    .map(|v| format!("'{}'", v))
                    .collect::<Vec<_>>()
                    .join(" | ")
            ));
        }

        self.includes.extend(&printer.includes);
        self.append(printer);
        self.println("}");

        if !msg.nested.is_empty() {
            self.println_and_indent(format!("namespace {} {{", msg_name));
            self.write_types(msg.nested.iter());
            self.outdent_and_println("}");
        }
    }

    /// Write a Proto enum typescript definitions
    fn write_enum(&mut self, e: &Enum) {
        for (name, value) in e.values.iter() {
            self.println(format!("{} = {},", name, value));
        }
    }

    /// create a copy of the current printer with a blank buffer
    fn printer_with_config(&self, indent: usize) -> Self {
        Self {
            buffer: String::new(),
            includes: HashSet::new(),
            config: self.config,
            indent,
        }
    }

    /// Print the content with a newline
    fn println<T: AsRef<str>>(&mut self, value: T) {
        for _ in 0..self.indent {
            self.buffer.push(' ');
        }
        self.buffer.push_str(value.as_ref());
        self.buffer.push('\n')
    }

    /// Print the content with a newline and increment indent
    fn println_and_indent<T: AsRef<str>>(&mut self, value: T) {
        self.println(value);
        self.indent += 2;
    }

    /// decrement indent and print the content with a newline
    fn outdent_and_println<T: AsRef<str>>(&mut self, value: T) {
        self.indent -= 2;
        self.println(value);
    }

    /// Print a blank line
    fn add_blank_line(&mut self) {
        self.buffer.push('\n');
    }

    /// Append the other printer content to self
    fn append(&mut self, other: Printer) {
        self.buffer.push_str(other.buffer.as_str())
    }

    /// Print a JSDoc comment
    fn print_comment(&mut self, md: &Metadata, include_link: bool) {
        let mut lines: Vec<Cow<str>> = match md.comment.as_ref() {
            Some(cmt) => cmt
                .text
                .split('\n')
                .map(|v| {
                    // replace "*/" as this breaks "*/" comments
                    let mut v = v.replace("*/", "*\\/");

                    // append a space if strings start with a / to avoid creating a breaking "*/"
                    if v.starts_with('/') {
                        v.replace_range(0..1, " ")
                    }

                    // ensure that each line starts
                    Cow::Owned(v)
                })
                .collect::<Vec<_>>(),
            None => Vec::new(),
        };

        if md.is_deprecated() {
            lines.push(" @deprecated".into())
        }

        if include_link {
            lines.push(
                format!(
                    " @link {url}/{path}#{line}",
                    url = self.config.root_url,
                    path = md.file_path.to_str().unwrap(),
                    line = md.line
                )
                .into(),
            );
        }

        if lines.is_empty() {
            return;
        }

        self.add_blank_line();
        self.println("/**");
        for line in lines {
            self.println(format!(" *{}", line))
        }

        self.println(" */");
    }

    /// Helper function that returns the type or the mapped Typescript if it exists
    fn get_type<'b>(&mut self, name: impl Into<&'b str>) -> &'b str {
        let name = name.into();
        match TYPE_MAPPING.get(name) {
            Some(t @ &"LongLike") => {
                self.includes.insert(LONG_LIKE_TYPE);
                t
            }
            Some(t) => t,
            None => &name[1..],
        }
    }

    /// Helper function that returns the rpc type
    fn rpc_type<'b>(&mut self, type_name: &'b str, is_streaming: bool) -> Cow<'b, str> {
        if is_streaming {
            self.includes.insert(OBSERVABLE_IMPORT);
            format!("Observable<{}>", self.get_type(type_name)).into()
        } else {
            self.get_type(type_name).into()
        }
    }
}

fn write_services<'a, F>(ns: &'a Namespace, writer: &mut F)
where
    F: FnMut(&'a Namespace, &'a str, &'a Rpc),
{
    for ns in ns.nested.values() {
        for service in ns.services.values() {
            for (method_name, rpc) in service.methods.iter() {
                writer(ns, method_name, rpc)
            }
        }

        write_services(ns, writer);
    }
}
