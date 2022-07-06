use std::collections::HashMap;


/// Simple mock DB that pretends a 'HashMap' is an async data source
pub struct DB {
    state: HashMap<DBKey, i64>,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DBKey(pub u32);

impl DB {
    pub fn init(state: HashMap<DBKey, i64>) -> Self {
        DB { state }
    }

    pub async fn get(&self, k: &DBKey) -> Result<i64, String> {
        self.state
            .get(k)
            .ok_or_else( || "mock db lookup failed".to_string())
            .map(|x| *x)
    }
}
