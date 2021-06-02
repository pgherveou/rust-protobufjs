use super::constants::TYPE_MAPPING;
use crate::{
    field::FieldRule, http_options::HTTPOptions, message::Message, metadata::Metadata,
    namespace::Namespace, r#enum::Enum, r#type::Type, service::Rpc, typescript::constants::*,
};
use convert_case::{Case, Casing};
use std::{
    borrow::Cow,
    collections::{BTreeMap, HashSet},
    fmt::Write,
};

/// PrintOptions let us configure How we want to print a Proto tree into a Typescript definition file
pub struct PrintConfig {
    pub root_url: String,
    pub print_bubble_client: bool,
    pub print_network_client: bool,
}

/// Printer serialize a Proto namespace into an internal buffer
pub struct Printer<'a> {
    /// The internal buffer used to build the TS definition
    buffer: String,

    /// Reference to the configuration
    config: &'a PrintConfig,

    /// List of extra types or imports to be added to the final output
    includes: HashSet<&'static str>,

    /// The indent level
    indent: usize,
}

/// write! wrapper that write to the printer buffer
macro_rules! writeln {
    ($printer:ident, $v:expr) => {{
        for _ in 0..$printer.indent {
            $printer.buffer.push(' ');
        }

        $printer.buffer.push_str($v);
        $printer.buffer.push('\n');
    }};
    ($printer:ident, $($arg:tt)*) => {{
        // print indent
        for _ in 0..$printer.indent {
            $printer.buffer.push(' ');
        }

        // print formatted string & newline
        write!(&mut $printer.buffer, $($arg)*).expect("Not written");
        $printer.buffer.push('\n')
    }};
}

/// write! wrapper that write and indent the printer
macro_rules! writeln_and_indent {
    ($printer:ident, $($arg:tt)*) => {{
        writeln!($printer, $($arg)*);
        $printer.indent += 2;
    }};
}

/// write! wrapper that outdent and write into the printer
macro_rules! outdent_and_writeln {
    ($printer:ident, $($arg:tt)*) => {{
        $printer.indent -= 2;
        writeln!($printer, $($arg)*);
    }};
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
        for_each_rpc(root, &mut |ns, method_name, rpc| {
            network_client_printer.write_network_client_rpc(ns, method_name, rpc);
            bubble_client_printer.write_bubble_client_rpc(ns, method_name, rpc);
        });

        // keep services definition that are defined in the config
        // and insert related import statements
        for (import, printer, enable) in [
            (
                NETWORK_CLIENT_IMPORT,
                &mut network_client_printer,
                self.config.print_network_client,
            ),
            (
                BUBBLE_CLIENT_IMPORT,
                &mut bubble_client_printer,
                self.config.print_bubble_client,
            ),
        ] {
            if enable && !printer.buffer.is_empty() {
                includes.insert(import);
            } else {
                printer.buffer.clear()
            }
        }

        // gather all includes
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
        .for_each(|import| writeln!(self, import));

        // print @lyft/bubble-client definitions
        if !bubble_client_printer.buffer.is_empty() {
            writeln_and_indent!(self, "declare module '@lyft/bubble-client' {");
            writeln_and_indent!(self, "interface Router {");
            self.append(bubble_client_printer);
            outdent_and_writeln!(self, "}");
            outdent_and_writeln!(self, "}");
        }

        // print @lyft/network-client definitions
        if !network_client_printer.buffer.is_empty() {
            writeln_and_indent!(self, "declare module '@lyft/network-client' {");
            writeln_and_indent!(self, "interface NetworkClient {");
            self.append(network_client_printer);
            outdent_and_writeln!(self, "}");
            outdent_and_writeln!(self, "}");
        }

        writeln!(self, "declare global {");

        // print global types from includes
        std::array::IntoIter::new([&LONG_LIKE_TYPE, &ANY_TYPE, &EMPTY])
            .filter(|val| includes.contains(*val))
            .for_each(|val| writeln!(self, val));

        self.add_blank_line();
        self.append(types_printer);
        writeln!(self, "}");
        self.buffer
    }

    /// Write @lyft/bubble-client typescript definitions
    fn write_bubble_client_rpc(&mut self, ns: &'a Namespace, method_name: &'a str, rpc: &'a Rpc) {
        self.print_comment(&rpc.md, true);
        let req = rpc.request_type.borrow();
        let req = self.rpc_type(req.as_str(), rpc.request_stream);

        let resp = rpc.response_type.borrow();
        let resp = self.rpc_type(resp.as_str(), rpc.response_stream);

        match HTTPOptions::from(&rpc.md.options) {
            Some(HTTPOptions {
                path,
                method,
                error_types,
            }) => {
                let code_error_tuples = error_types
                    .iter()
                    .map(|e| e.as_string())
                    .collect::<Vec<_>>()
                    .join(" | ");

                writeln_and_indent!(self, "{}(", method.to_lowercase());
                writeln!(self, "path: '{}',", path);

                writeln!(
                    self,
                    "handler: RouteHandler<{}, {}, {}>",
                    req, resp, code_error_tuples,
                );
                outdent_and_writeln!(self, "): void");
            }
            None => {
                writeln_and_indent!(self, "grpc(");
                writeln!(self, "path: '/{}/{}',", ns.path.join("."), method_name);
                writeln!(
                    self,
                    "handler: RouteHandler<{}, {}, [code: number, body: string]>",
                    req, resp
                );
                outdent_and_writeln!(self, "): void");
            }
        }
    }

    /// Write @lyft/network-client typescript definitions
    fn write_network_client_rpc(&mut self, ns: &'a Namespace, method_name: &'a str, rpc: &'a Rpc) {
        let req = rpc.request_type.borrow();
        let req = self.rpc_type(req.as_str(), rpc.request_stream);

        let resp = rpc.response_type.borrow();
        let resp = self.rpc_type(resp.as_str(), rpc.response_stream);

        self.print_comment(&rpc.md, true);

        match HTTPOptions::from(&rpc.md.options) {
            Some(HTTPOptions { path, method, .. }) => {
                writeln_and_indent!(self, "{method}(", method = method.to_lowercase());
                writeln!(self, "path: '{path}'", path = path);
                outdent_and_writeln!(self, "): HTTPResource<{}, {}>", req, resp);
            }
            None => {
                writeln_and_indent!(self, "grpc(");
                writeln!(self, "path: '/{}/{}'", ns.path.join("."), method_name);
                outdent_and_writeln!(
                    self,
                    "): GRPCResource<{}, {}, [code: number, body: string]>): void",
                    req,
                    resp
                );
            }
        }
    }

    /// Write namespace typescript definitions
    fn write_namespaces(&mut self, namespaces: &'a BTreeMap<String, Namespace>) {
        for (name, ns) in namespaces {
            writeln_and_indent!(self, "namespace {} {{", name);
            self.write_types(ns.types.iter());
            self.write_namespaces(&ns.nested);
            outdent_and_writeln!(self, "}");
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
                    writeln_and_indent!(self, "const enum {} {{", name);
                    self.write_enum(e);
                    outdent_and_writeln!(self, "}");
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
            match (&field.key_type, &field.rule) {
                (Some(key), _) => {
                    writeln!(printer, "{}?: {{ [key: {}]: {} }}", name, key, type_name);
                }
                (None, Some(FieldRule::Repeated)) => {
                    writeln!(printer, "{}?: Array<{}>", name, type_name);
                }
                (None, _) => writeln!(printer, "{}?: {}", name, type_name),
            };
        }

        match generic_constraints.len() {
            0 => match msg.fields.len() {
                0 => {
                    self.includes.insert(EMPTY);
                    writeln!(self, "interface {} extends Empty {{", msg_name)
                }
                _ => writeln!(self, "interface {} {{", msg_name),
            },
            _ => writeln!(
                self,
                "interface {}<{}> {{",
                msg_name,
                generic_constraints.join(",")
            ),
        }

        for (name, oneof) in msg.oneofs.iter() {
            printer.print_comment(&oneof.md, false);
            writeln!(
                printer,
                "{}?: Extract<keyof {}, {}>",
                name,
                msg_name,
                oneof
                    .values
                    .iter()
                    .map(|v| format!("'{}'", v))
                    .collect::<Vec<_>>()
                    .join(" | ")
            );
        }

        self.includes.extend(&printer.includes);
        self.append(printer);
        writeln!(self, "}");

        if !msg.nested.is_empty() {
            writeln_and_indent!(self, "namespace {} {{", msg_name);
            self.write_types(msg.nested.iter());
            outdent_and_writeln!(self, "}");
        }
    }

    /// Write a Proto enum typescript definitions
    fn write_enum(&mut self, e: &Enum) {
        for (name, value) in e.values.iter() {
            writeln!(self, "{} = {},", name, value);
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
        writeln!(self, "/**");
        for line in lines {
            writeln!(self, " *{}", line)
        }

        writeln!(self, " */");
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

// Helper function that execute recursively for each rpc in a namespace
fn for_each_rpc<'a, F>(ns: &'a Namespace, callback: &mut F)
where
    F: FnMut(&'a Namespace, &'a str, &'a Rpc),
{
    for ns in ns.nested.values() {
        for service in ns.services.values() {
            for (method_name, rpc) in service.methods.iter() {
                callback(ns, method_name, rpc)
            }
        }

        for_each_rpc(ns, callback);
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        parser::test_util::parse_test_file,
        typescript::serializer::{PrintConfig, Printer},
    };
    use indoc::indoc;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_generate_typescript_definition() {
        let root = parse_test_file(indoc! {r#"
        package pb.hello;
        
        service HelloWorld {
          rpc LotsOfGreetings(stream SayHelloRequest) returns (SayHelloResponses) {}
          rpc SayHello (SayHelloRequest) returns (SayHelloResponse) {
              option (pgm.http.rule) = { GET: "/hello/<string:name>" };
          }
        }
        
        message SayHelloRequest {
          string name = 1;
        }
        
        message SayHelloResponse {
          string hello = 1;
        }
        
        message SayHelloResponses {
          repeated SayHelloResponse responses = 1;
        }
        "#});

        let config = PrintConfig {
            root_url: "https://github.com/lyft/idl/blob/master/protos".into(),
            print_bubble_client: true,
            print_network_client: true,
        };

        let printer = Printer::new(&config);
        let output = printer.into_string(&root);

        let result = indoc! {r#"
        import { Observable } from 'rxjs'
        import { RouteHandler } from '@lyft/bubble-client'
        import { GRPCResource, HTTPResource } from '@lyft/network-client'
        declare module '@lyft/bubble-client' {
          interface Router {
        
            /**
             * @link https://github.com/lyft/idl/blob/master/protos/test.proto#4
             */
            grpc(
              path: '/pb.hello/LotsOfGreetings',
              handler: RouteHandler<Observable<pb.hello.SayHelloRequest>, pb.hello.SayHelloResponses, [code: number, body: string]>
            ): void
        
            /**
             * @link https://github.com/lyft/idl/blob/master/protos/test.proto#5
             */
            get(
              path: '/hello/:name',
              handler: RouteHandler<pb.hello.SayHelloRequest, pb.hello.SayHelloResponse, [code: number, body: unknown]>
            ): void
          }
        }
        declare module '@lyft/network-client' {
          interface NetworkClient {
        
            /**
             * @link https://github.com/lyft/idl/blob/master/protos/test.proto#4
             */
            grpc(
              path: '/pb.hello/LotsOfGreetings'
            ): GRPCResource<Observable<pb.hello.SayHelloRequest>, pb.hello.SayHelloResponses, [code: number, body: string]>): void
        
            /**
             * @link https://github.com/lyft/idl/blob/master/protos/test.proto#5
             */
            get(
              path: '/hello/:name'
            ): HTTPResource<pb.hello.SayHelloRequest, pb.hello.SayHelloResponse>
          }
        }
        declare global {
        
          namespace pb {
            namespace hello {
        
              /**
               * @link https://github.com/lyft/idl/blob/master/protos/test.proto#10
               */
              interface SayHelloRequest {
                name?: string
              }
        
              /**
               * @link https://github.com/lyft/idl/blob/master/protos/test.proto#14
               */
              interface SayHelloResponse {
                hello?: string
              }
        
              /**
               * @link https://github.com/lyft/idl/blob/master/protos/test.proto#18
               */
              interface SayHelloResponses {
                responses?: Array<pb.hello.SayHelloResponse>
              }
            }
          }
        }
        "#};

        assert_eq!(output, result);
    }
}
