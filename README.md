# Book Reader

PRE-ALPHA Program.

Lots of `.unwrap()`s which need to be removed.

My intention will be to make it [Plex](https://plex.tv) for Books.


## Running/Building

To run and build the application you need to do the following:

[Install Rust](https://www.rust-lang.org/). It's used for coding this whole application.

[Install Trunk](https://trunkrs.dev/#install). It's used for building the frontend.


### Frontend Build:

Set your CWD inside `crates/frontend` and run `trunk build`

### Backend Build:

Set your CWD inside `crates/backend` and run `cargo build`

### Running

If you'd now like to run the application CWD inside `crates/backend` and run `cargo run`

The server will now be hosted on `127.0.0.1:8084`