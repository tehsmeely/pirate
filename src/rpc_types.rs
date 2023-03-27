use crate::core::RpcType;

use serde::{Deserialize, Serialize};

impl<T> RpcType for T where T: Clone + Serialize + for<'de> Deserialize<'de> + 'static {}
