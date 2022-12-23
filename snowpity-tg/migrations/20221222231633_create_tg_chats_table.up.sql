create table if not exists tg_chats(
    id bigint not null,

    updated_at timestamp with time zone not null default (now()::timestamp),
    created_at timestamp with time zone not null default (now()::timestamp),

    created_by bigint not null,

    captcha_enabled boolean not null default false,

    constraint tg_chats_pk primary key (id)
);
