// TODO: Was going to create my own. Decided against it for now.


use std::io::{Read, Seek, SeekFrom};

use crate::Result;


pub struct MobiReader<D: Read + Seek> {
    data: D
}

impl<D: Read + Seek> MobiReader<D> {
    pub fn new(data: D) -> Result<Self> {
        let mut this = Self {
            data
        };

        this.verify()?;

        Ok(this)
    }

    fn verify(&mut self) -> Result<()> {
        let name = read_next_to_vec(&mut self.data, 32)?;
        println!("{:?}", std::str::from_utf8(&name));

        // let compression = read_u16(&mut self.data)?;
        // let unused = read_u16(&mut self.data)?;

        // println!("{:?}", compression);
        // println!("{:?}", unused);

        Ok(())
    }
}

fn read_next_to_vec<D: Read + Seek>(data: &mut D, len: usize) -> Result<Vec<u8>> {
    let mut read = vec![0; len];

    data.read_exact(&mut read)?;

    data.seek(SeekFrom::Current(len as i64))?;

    Ok(read.to_vec())
}

fn read_u16<D: Read + Seek>(data: &mut D) -> Result<u16> {
    let mut bytes = [0; 2];
    data.read_exact(&mut bytes)?;
    Ok(u16::from_be_bytes(bytes))
}

fn read_u8<D: Read + Seek>(data: &mut D) -> Result<u8> {
    let mut bytes = [0; 1];
    data.read_exact(&mut bytes)?;
    Ok(u8::from_be_bytes(bytes))
}