# Book Reader

PRE-ALPHA Program.

My intention will be to make it [Plex](https://plex.tv) for Books.

TODO:
 - Utilize sqlite transactions for multi table changes.
 - Implement database migrations


# Running/Building

To run and build the application you need to do the following:

Minimum Rust Version 1.65

[Install Rust](https://www.rust-lang.org/). It's used for coding this whole application.

[Install Trunk](https://trunkrs.dev/#install). It's used for building the frontend.


## Backend:
Inside **root folder** execute these commands:
```bash
cargo run --bin books-backend
```

The server will now be hosted on `127.0.0.1:8084`

## Frontend:
Execute one of these commands

To build:
```bash
cd crates/frontend
trunk build --public-url "/dist" -d "../../app/public/dist"
```

To build and watch:
```bash
cd crates/frontend
trunk watch --public-url "/dist" -d "../../app/public/dist"
```

# Packaging
Packaging will store the frontend in to the reader executable and extract files when ran.

## Frontend:
Execute this first

To build:
```bash
cd crates/frontend
trunk build --release --public-url "/dist" -d "../../app/public/dist"
```

## Backend:
Inside **root folder** execute these commands:

```bash
cargo build --bin backend-bundled --release --features=bundled
```

The packaged executable will now be inside target/release


# Gallery


## Overview

![Overview](https://i.thick.at/RememberedReginaSpektor800.png)


## Basic library viewer.

![Home](https://i.thick.at/PublishedFarEastMovement196.jpeg)


## Basic book viewer.

![Book View](https://i.thick.at/OverviolentGratiano156.png)


## Basic book reader.

![Book Reader](https://i.thick.at/AntimodernistWildBoar735.png)


## It can even go fullscreen!

![Book Reader Fullscreen](https://i.thick.at/EmeticEverythingEverything016.png)