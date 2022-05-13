use serde::{Serialize, Deserialize};


pub mod setup;
mod thumbnail;
mod source;
mod edit;

pub use thumbnail::*;
pub use source::*;
pub use edit::*;




#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Either<A, B> {
	Left(A),
	Right(B),
}