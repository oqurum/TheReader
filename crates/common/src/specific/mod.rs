mod thumbnail;
mod source;

use serde::{Serialize, Deserialize};
pub use thumbnail::*;
pub use source::*;




#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Either<A, B> {
	Left(A),
	Right(B),
}