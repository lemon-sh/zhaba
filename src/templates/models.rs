#[derive(Debug)]
pub struct Post {
    pub id: u64,
    pub content: String,
    pub image: Option<String>,
    pub ip: String,
    // ASN + MNT
    pub whois: Option<(u32, String)>,
    pub time: String,
}
