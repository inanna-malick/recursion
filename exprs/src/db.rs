use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DBKey(pub u32);

pub struct DB {
    state: HashMap<DBKey, i64>,
}

impl DB {
    pub fn init(state: HashMap<DBKey, i64>) -> Self {
        DB { state }
    }

    pub async fn get(&self, k: &DBKey) -> Result<i64, String> {
        self.state
            .get(k)
            .ok_or("mock db lookup failed".to_string())
            .map(|x| *x)
    }
}
