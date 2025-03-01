
use std::io::Read;
use std::path::Path;
use std::fs::File;

use std::net::TcpStream;

#[derive(Debug, Clone, PartialEq)]
pub enum NetworkError {

}

pub const NET_BUFF_SIZE: usize = 4096;

pub fn read_file_for_network(path: &Path, into: &mut Vec<[u8; NET_BUFF_SIZE]>) -> Result<(),std::io::Error> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    split_binary_for_network(contents.as_bytes(), into);
    Ok(())
}

pub fn split_binary_for_network(contents: &[u8], dest: &mut Vec<[u8; NET_BUFF_SIZE]>) {
    let evaluated = contents.windows(NET_BUFF_SIZE);
    todo!()

}

pub fn receive_buffer(dest: &mut Vec<u8>, frame_count: usize, sock: TcpStream) -> bool {
    todo!()
}