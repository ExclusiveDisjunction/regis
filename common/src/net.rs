use std::ops::DerefMut;
use std::path::Path;

use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::pin;

pub const NET_BUFF_SIZE: usize = 4096;

pub async fn read_file_contents<P>(path: P) -> Result<String, std::io::Error> where P: AsRef<Path> {
    let mut file = File::open(path).await?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).await?;

    Ok(contents)
}

pub async fn send_buffer<T>(src: &[u8], sock: &mut T) -> Result<(), std::io::Error> where T: AsyncWriteExt + Unpin {
    let mut total_written = 0;
    pin!(sock);
    while total_written < src.len() {
        let written = sock.write(&src[total_written..]).await?;
        total_written += written;
    }

    Ok(())
}
pub async fn receive_buffer<T>(dest: &mut Vec<u8>, sock: &mut T) -> Result<(), std::io::Error> where T: AsyncReadExt + Unpin {
    dest.clear();
    let mut temp_buffer = Box::new([0; NET_BUFF_SIZE]);

    pin!(sock);

    loop {
        let bytes_read = sock.read(temp_buffer.deref_mut()).await?;

        if bytes_read == 0 {
            break; //Connection closed or no more data
        }

        dest.extend_from_slice(&temp_buffer[..bytes_read]);
    }

    Ok(())
}