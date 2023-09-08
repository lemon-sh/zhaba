use color_eyre::{eyre::eyre, Result};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};

#[derive(Debug)]
pub struct WhoisResult {
    pub asn: u32,
    pub mnt: String,
}

pub async fn whois(server: &str, query: &str) -> Result<Option<WhoisResult>> {
    if server == "!" {
        return Ok(Some(WhoisResult {
            asn: 4242426969,
            mnt: "MIETEK-MNT".into(),
        }));
    }
    let mut stream = TcpStream::connect(server).await?;
    let (read, mut write) = stream.split();
    let mut lines = BufReader::new(read).lines();

    write.write_all(query.as_bytes()).await?;
    write.write_all(&[b'\n']).await?;

    loop {
        if let Some(line) = lines.next_line().await? {
            if line.starts_with("% Information related to 'route/") {
                break;
            }
        } else {
            return Ok(None);
        }
    }

    let mut asn = None;
    let mut mnt = None;

    while let Some(line) = lines.next_line().await? {
        if let Some((key, value)) = line.split_once(':') {
            match key {
                "origin" => {
                    let asnstr = value
                        .trim()
                        .get(2..)
                        .ok_or_else(|| eyre!("Invalid ASN format from whois: {value}"))?;
                    asn = Some(asnstr.parse()?);
                }
                "mnt-by" => mnt = Some(value.trim().to_string()),
                _ => {}
            }
        }
    }

    if let (Some(asn), Some(mnt)) = (asn, mnt) {
        Ok(Some(WhoisResult { asn, mnt }))
    } else {
        Ok(None)
    }
}
