pub static INSERT_POST: &str = "insert into posts(content,image,ip,asn,mnt,reply,board) values (?,?,?,?,?,?,(select id from boards where name = ?))";
pub static DELETE_POST: &str = "delete from posts where id = ? returning image";
pub static SELECT_POSTS_BOARD_RANGE: &str = "select post.id, post.content, post.image, post.ip, post.asn, post.mnt, post.reply, post.time, post.board, reply.id, reply.time, reply.board, reply_board.name from posts as post left join posts as reply on post.reply = reply.id left join boards as reply_board on reply.board = reply_board.id where post.board = ? and post.time between ? and ? order by post.time desc";
pub static CHECK_REPLY: &str = "select 1 from posts where id = ?";

pub static INSERT_BOARD: &str = "insert into boards(name,description,color) values(?,?,?)";
pub static DELETE_BOARD: &str = "delete from boards where id = ?";
pub static SELECT_BOARDS: &str = "select * from boards";
pub static SELECT_BOARD_BY_NAME: &str = "select * from boards where name = ?";
pub static UPDATE_BOARD: &str =
    "update boards set name = ?, description = ?, color = ? where id = ?";
