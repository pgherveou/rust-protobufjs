use std::borrow::Cow;

use crate::metadata::ProtoOption;
use lazy_static::lazy_static;
use regex::Regex;

#[derive(Debug, PartialEq, Eq)]
pub struct HTTPErrorType<'a> {
    code: &'a str,
    type_name: &'a str,
}

impl<'a> HTTPErrorType<'a> {
    pub fn as_string(&self) -> String {
        format!("[code: {}, body: {}]", self.code, self.type_name)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct HTTPOptions<'a> {
    pub path: Cow<'a, str>,
    pub method: &'a str,
    pub error_types: Vec<HTTPErrorType<'a>>,
}

impl<'a> HTTPOptions<'a> {
    pub fn from(raw_options: &'a [ProtoOption]) -> Option<Self> {
        let mut path = None;
        let mut method = None;
        let mut error_types = Vec::new();
        let mut default_error = None;

        for option in raw_options {
            let option = option.iter().map(String::as_str).collect::<Vec<_>>();

            match option[..] {
                ["pgm.http.rule", rule_method, rule_path] => {
                    path.replace(rule_path);
                    method.replace(rule_method);
                }
                ["pgm.error.rule", "default_error_type", type_name, ..] => {
                    default_error.replace(HTTPErrorType {
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
                    default_error.replace(HTTPErrorType {
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
            (Some(path), Some(method)) => {
                if let Some(default_error) = default_error {
                    error_types.push(default_error)
                }

                if error_types.is_empty() {
                    error_types.push(HTTPErrorType {
                        code: "number",
                        type_name: "unknown",
                    })
                }

                lazy_static! {
                    // replace /api/<foo:string> => /api/:foo
                    static ref HTTP_REGEX: Regex = Regex::new("(<.*?:(.*?)>)").unwrap();
                }

                // let path = HTTP_REGEX.replace_all(path, ":$2");
                let path = HTTP_REGEX.replace_all(path, ":$2");

                Some(HTTPOptions {
                    path,
                    method,
                    error_types,
                })
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        file_parser::FileParser,
        http_options::{HTTPErrorType, HTTPOptions},
        metadata::ProtoOption,
    };
    use indoc::indoc;
    use std::path::PathBuf;

    fn get_options(text: &str) -> Vec<ProtoOption> {
        let file_path: PathBuf = "test.proto".into();
        let parser = FileParser::new(file_path, text.chars());
        let mut ns = parser.parse().expect("failed to parse content");

        let hello = ns
            .services
            .remove("HelloWorld")
            .expect("HelloWorld service not found")
            .methods
            .remove("GetHello")
            .expect("GetHello method not found");

        hello.md.options
    }

    macro_rules! test_http_options {
        ($name:ident, $text:expr, $expected:expr) => {
            #[test]
            fn $name() {
                let options = get_options($text);
                let http_options =
                    HTTPOptions::from(&options).expect("failed to parse HTTPOptions");

                assert_eq!(http_options, $expected)
            }
        };
    }

    test_http_options!(
        test_legacy_parsing,
        indoc! {r#"
        service HelloWorld {
          rpc GetHello (SayHelloRequest) returns (SayHelloResponse) {
            option (http.http_options).path = "/hello";
            option (http.http_options).method = "GET";
            option (http.http_options).error_type = "DefaultError";
            option (http.http_options).error_overrides = {code: 404, type: "404Error"};
          }
        }
        "#},
        HTTPOptions {
            method: "GET",
            path: "/hello".into(),
            error_types: vec![
                HTTPErrorType {
                    code: "404",
                    type_name: "404Error"
                },
                HTTPErrorType {
                    code: "number",
                    type_name: "DefaultError",
                },
            ]
        }
    );

    test_http_options!(
        test_pgm_parsing,
        indoc! {r#"
        service HelloWorld {
          rpc GetHello (SayHelloRequest) returns (SayHelloResponse) {
              option (pgm.http.rule) = { GET: "/hello" };
              option (pgm.error.rule) = {
                  default_error_type: "DefaultError",
                  error_override {
                    code: 404,
                    type: "404Error",
                  }                  
              };
          }
        }
        "#},
        HTTPOptions {
            method: "GET",
            path: "/hello".into(),
            error_types: vec![
                HTTPErrorType {
                    code: "404",
                    type_name: "404Error"
                },
                HTTPErrorType {
                    code: "number",
                    type_name: "DefaultError",
                },
            ]
        }
    );

    test_http_options!(
        test_dynamic_path,
        indoc! {r#"
        service HelloWorld {
          rpc GetHello (SayHelloRequest) returns (SayHelloResponse) {
              option (pgm.http.rule) = { GET: "/hello/<string:one>/<string:two>" };
          }
        }
        "#},
        HTTPOptions {
            method: "GET",
            path: "/hello/:one/:two".into(),
            error_types: vec![HTTPErrorType {
                code: "number",
                type_name: "unknown",
            },]
        }
    );

    #[test]
    fn test_no_http_options() {
        let options = get_options(indoc! {r#"
            service HelloWorld {
                rpc GetHello (SayHelloRequest) returns (SayHelloResponse) {}
            }
        "#});

        assert_eq!(HTTPOptions::from(&options), None)
    }
}
