# Book Reader

An organizer for your books that allows you to read them in your browser. With a focus on simplicity and ease of use. Currently only supports epub (cbz is almost impl. too) files.


My intention will be to make it [Plex](https://plex.tv) for Books.

TODO:
 - Better Separation of books' Sections
 - Utilize sqlite transactions for multi table changes.
 - Implement database migrations
 - Cache & Clear external searches for each task ran


# Running/Building

To run and build the application you need to do the following:

[Install Rust](https://www.rust-lang.org/). It's used for coding this whole application.

[Install Trunk](https://trunkrs.dev/#install). It's used for building the frontend.

## Git
Import the submodules
```bash
git submodule update --init
```

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

## Mobile View

![Overview](https://i.thick.at/BustedButterfly562.png)
![Library](https://i.thick.at/AttentRedTornado025.png)
![Book View](https://i.thick.at/MonoculturalYarn453.png)
![Book Reader](https://i.thick.at/MoonishBorsec836.png)

## Desktop View

### Overview

![Overview](https://i.thick.at/DishonorableRJD2117.png)

### Basic library viewer.

![Home](https://i.thick.at/AntagonizingFleance243.jpeg)

### Basic book viewer.

![Book View](https://i.thick.at/AntispasmodicCodyJinks427.png)

### Basic book reader.

![Book Reader](https://i.thick.at/SightlyWalkersShots711.png)

### It can even go fullscreen!

![Book Reader Fullscreen](https://i.thick.at/EmeticEverythingEverything016.png)
