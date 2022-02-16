use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct SendItem {
    name: String,
    full_path: String,
    size: i64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SendList(Vec<SendItem>);

impl AsRef<Vec<SendItem>> for SendList {
    fn as_ref(&self) -> &Vec<SendItem> {
        &self.0
    }
}
