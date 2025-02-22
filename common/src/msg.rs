use serde::{Serialize, Deserialize};

use std::fmt::Debug;

pub trait MessageBasis: Serialize + for<'de> Deserialize<'de> + PartialEq + Clone + Debug {

}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum Messages {

}
impl MessageBasis for Messages {

}

#[derive(Serialize, Deserialize)]
pub enum Direction {
    Request,
    Response
}

#[derive(Serialize, Deserialize)]
pub struct MessageWrap {
    inner: Messages,
    dir: Direction
}