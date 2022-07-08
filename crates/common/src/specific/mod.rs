use serde::{Serialize, Deserialize};


pub mod setup;
mod thumbnail;
mod source;
mod edit;
mod id;

pub use thumbnail::*;
pub use source::*;
pub use edit::*;
pub use id::*;



#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Either<A, B> {
	Left(A),
	Right(B),
}