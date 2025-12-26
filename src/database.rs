use std::{collections::HashMap, net::IpAddr};

use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
pub struct DataBase {
    pub user_name: String,
    pub alias_table: HashMap<String, IpAddr>,
    pub folder: String,
}

