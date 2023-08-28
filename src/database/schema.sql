pragma journal_mode = wal;
pragma synchronous = 0;

create table if not exists posts(
    id integer primary key,
    content text not null,
    image text,
    ip text not null,
    asn integer,
    mnt text,
    time integer default (strftime('%s','now')),
    board integer not null,
    foreign key (board) references boards(id) on delete cascade
);

create table if not exists boards(
    id integer primary key,
    name text not null unique,
    description text not null,
    color integer not null
);

create index if not exists idx_board_time on posts(time, board);
