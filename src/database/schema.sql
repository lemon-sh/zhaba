pragma journal_mode = wal;
pragma synchronous = 0;

create table if not exists posts (
    id integer primary key,
    post text not null,
    ip text not null,
    show_ip boolean not null,
    image text,
    time integer not null
);
