use std::fs;
use std::path::Path;

pub struct AttachedFile {
    pub path: String,
    pub name: String,
    content: Vec<u8>,
}

impl From<&str> for AttachedFile {
    fn from(path: &str) -> Self {
        let content = fs::read(path);

        let content: Vec<u8> = match content {
            Ok(inner) => inner,
            Err(_) => error!("Failed to read attatched file {}", path),
        };

        let name = Path::new(path)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        AttachedFile {
            content,
            name,
            path: path.to_string(),
        }
    }
}

impl AttachedFile {
    pub fn get_button_html(&self) -> String {
        let data = base64::encode(&self.content);

        format!(" <a href=\"data:application/octet-stream;base64,{}\" download=\"{}\" class=\"btn btn-success\">{}</a> ", data, self.name, self.name)
    }
}
