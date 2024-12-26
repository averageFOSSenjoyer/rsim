use crate::error::SimError::SimManagerError;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::sync::PoisonError;

#[derive(Debug)]
pub enum SimError {
    SimManagerError,
}

impl Display for SimError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "SimError")
    }
}

impl Error for SimError {}

impl<T> From<PoisonError<T>> for SimError {
    fn from(_value: PoisonError<T>) -> Self {
        SimManagerError
    }
}
