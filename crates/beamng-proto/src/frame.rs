use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::error::{BngError, Result};

/// Read a single length-prefixed frame from the reader.
///
/// Wire format: 4-byte big-endian length prefix followed by `length` bytes of payload.
pub async fn read_frame<R: AsyncReadExt + Unpin>(reader: &mut R) -> Result<Vec<u8>> {
    let len = reader.read_u32().await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::UnexpectedEof {
            BngError::Disconnected("Connection closed while reading frame header".into())
        } else {
            BngError::Io(e)
        }
    })?;

    let len = len as usize;
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::UnexpectedEof {
            BngError::Disconnected("Connection closed while reading frame body".into())
        } else {
            BngError::Io(e)
        }
    })?;

    Ok(buf)
}

/// Write a single length-prefixed frame to the writer.
pub async fn write_frame<W: AsyncWriteExt + Unpin>(writer: &mut W, data: &[u8]) -> Result<()> {
    let len = data.len() as u32;
    writer.write_u32(len).await?;
    writer.write_all(data).await?;
    writer.flush().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_roundtrip() {
        let payload = b"hello world";
        let mut buf = Vec::new();
        write_frame(&mut buf, payload).await.unwrap();

        assert_eq!(buf.len(), 4 + payload.len());
        // Verify big-endian length prefix
        let len = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
        assert_eq!(len as usize, payload.len());

        let mut cursor = &buf[..];
        let result = read_frame(&mut cursor).await.unwrap();
        assert_eq!(result, payload);
    }

    #[tokio::test]
    async fn test_empty_payload() {
        let payload = b"";
        let mut buf = Vec::new();
        write_frame(&mut buf, payload).await.unwrap();

        let mut cursor = &buf[..];
        let result = read_frame(&mut cursor).await.unwrap();
        assert!(result.is_empty());
    }
}
