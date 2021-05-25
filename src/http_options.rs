pub struct HTTPErrorType<'a> {
    code: &'a str,
    type_name: &'a str,
}

impl<'a> HTTPErrorType<'a> {
    pub fn as_string(&self) -> String {
        format!("[code: {}, body: {}]", self.code, self.type_name)
    }
}

pub struct HTTPOptions<'a> {
    pub path: &'a str,
    pub method: &'a str,
    pub error_types: Vec<HTTPErrorType<'a>>,
}

impl<'a> HTTPOptions<'a> {
    pub fn from(raw_options: &'a [Vec<String>]) -> Option<Self> {
        let mut path = None;
        let mut method = None;
        let mut error_types = Vec::new();

        for option in raw_options {
            let option = option.iter().map(String::as_str).collect::<Vec<_>>();

            match option[..] {
                ["pgm.http.rule", rule_method, rule_path] => {
                    path.replace(rule_path);
                    method.replace(rule_method);
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
                path,
                method,
                error_types,
            }),
            _ => None,
        }
    }
}
