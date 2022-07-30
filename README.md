# Book Reader

PRE-ALPHA Program.

Lots of `.unwrap()`s which need to be removed.

My intention will be to make it [Plex](https://plex.tv) for Books.

Todo:
 - REDO SCSS and move into common git repo to share with reader
 - and more in TODO comments throughout the code


## Running/Building

To run and build the application you need to do the following:

[Install Rust](https://www.rust-lang.org/). It's used for coding this whole application.

[Install Trunk](https://trunkrs.dev/#install). It's used for building the frontend.


## Backend:
Inside **root folder** execute these commands:
```bash
cargo run --bin books-backend
```

The server will now be hosted on `127.0.0.1:8084`

## Frontend:
Inside **crates/frontend folder** execute one of these commands

To build:
```bash
trunk build --public-url "/dist" -d "../../app/public/dist"
```

To build and watch:
```bash
trunk watch --public-url "/dist" -d "../../app/public/dist"
```
