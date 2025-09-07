
//! Special marker trait for publishing. Implement this trait for your type if it is not [`Copy']
//! but can be cloned cheaply.
pub trait CheapClone : Clone {}
