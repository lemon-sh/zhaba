pub fn imghdr(buf: &[u8]) -> Option<&'static str> {
    // maybe also add AVIF/HEIC support
    match () {
        _ if &buf[..8] == b"\x89PNG\r\n\x1a\n" => Some(".png"),
        _ if (&buf[6..10] == b"JFIF") || (&buf[6..10] == b"Exif") => Some(".jpg"),
        _ if (&buf[..6] == b"GIF87a") || (&buf[..6] == b"GIF89a") => Some(".gif"),
        _ => None,
    }
}
