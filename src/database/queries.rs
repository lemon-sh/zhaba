pub const INSERT_POST: &str = "insert into posts(content, image, ip, asn, mnt) values (?,?,?,?,?)";
pub const SELECT_POSTS: &str = "select * from posts where id > ? order by id desc limit ?";
