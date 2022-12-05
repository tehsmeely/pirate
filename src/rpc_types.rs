use crate::core::{RpcType, ToFromBytes};
use crate::error::RpcResult;
use crate::{Bytes, OwnedBytes};

use serde::{Deserialize, Serialize};

impl<T> RpcType for T where T: Clone + Serialize + for<'de> Deserialize<'de> + 'static {}
