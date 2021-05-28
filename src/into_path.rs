/// Blanket trait to convert path String to Vec
pub trait IntoPath {
    fn into_path(self) -> Vec<String>;
}

impl<T: AsRef<str>> IntoPath for T {
    fn into_path(self) -> Vec<String> {
        self.as_ref().split('.').map(|v| v.to_string()).collect()
    }
}

/// Blanket trait to convert Vec to Path string
pub trait ToPath {
    fn to_path_string(self) -> String;
}

impl ToPath for Vec<&str> {
    fn to_path_string(mut self) -> String {
        self.insert(0, "");
        self.join(".")
    }
}
