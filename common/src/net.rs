use std::ops::DerefMut;
use std::path::Path;

use std::io::{Read, Write};
use std::fs::File;

use tokio::fs::File as AsyncFile;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::pin;

pub const NET_BUFF_SIZE: usize = 4096;

pub fn read_file_contents<P>(path: P) -> Result<String, std::io::Error> where P: AsRef<Path> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    Ok(contents)
}

pub fn send_buffer<T>(src: &[u8], sock: &mut T) -> Result<(), std::io::Error> where T: Write {
    let mut total_written = 0;
    let length = (src.len() as u32).to_be_bytes();
    sock.write_all(&length)?;
    while total_written < src.len() {
        let written = sock.write(&src[total_written..])?;
        total_written += written;
    }

    Ok(())
}
pub fn receive_buffer<T>(dest: &mut Vec<u8>, sock: &mut T) -> Result<(), std::io::Error> where T: Read {
    dest.clear();
    let mut len_buff = [0u8, 4];
    sock.read_exact(&mut len_buff)?;
    let mut temp_buffer = Box::new([0; NET_BUFF_SIZE]);

    loop {
        let bytes_read = sock.read(temp_buffer.deref_mut())?;

        if bytes_read == 0 {
            break; //Connection closed or no more data
        }

        dest.extend_from_slice(&temp_buffer[..bytes_read]);
    }

    Ok(())
}

pub async fn read_file_contents_async<P>(path: P) -> Result<String, std::io::Error> where P: AsRef<Path> {
    let mut file = AsyncFile::open(path).await?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).await?;

    Ok(contents)
}

pub async fn send_buffer_async<T>(src: &[u8], sock: &mut T) -> Result<(), std::io::Error> where T: AsyncWriteExt + Unpin {
    let mut total_written = 0;

    let length = (src.len() as u32).to_be_bytes();
    sock.write_all(&length).await?;

    pin!(sock);
    while total_written < src.len() {
        let written = sock.write(&src[total_written..]).await?;
        total_written += written;
    }

    Ok(())
}
pub async fn receive_buffer_async<T>(dest: &mut Vec<u8>, sock: &mut T) -> Result<(), std::io::Error> where T: AsyncReadExt + Unpin {
    dest.clear();

    pin!(sock);

    let mut len_buff = [0u8, 4];
    let mut temp_buffer = Box::new([0; NET_BUFF_SIZE]);
    sock.read_exact(&mut len_buff).await?;

    loop {
        let bytes_read = sock.read(temp_buffer.deref_mut()).await?;

        if bytes_read == 0 {
            break; //Connection closed or no more data
        }

        dest.extend_from_slice(&temp_buffer[..bytes_read]);
    }

    Ok(())
}