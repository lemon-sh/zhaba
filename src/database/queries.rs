pub static INSERT_POST: &str = "insert into posts(content,image,ip,asn,mnt,board) values (?,?,?,?,?,(select id from boards where name = ?))";
pub static DELETE_POST: &str = "delete from posts where id = ? returning image";
pub static SELECT_POSTS_BOARD_RANGE: &str = "select id,content,image,ip,asn,mnt,time from posts where board = ? and time between ? and ? order by time desc";

pub static INSERT_BOARD: &str = "insert into boards(name,description,color) values(?,?,?)";
pub static DELETE_BOARD: &str = "delete from boards where id = ?";
pub static SELECT_BOARDS: &str = "select * from boards";
pub static SELECT_BOARD_BY_NAME: &str = "select * from boards where name = ?";
pub static UPDATE_BOARD: &str =
    "update boards set name = ?, description = ?, color = ? where id = ?";
