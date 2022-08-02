# Book Reader

PRE-ALPHA Program.

Lots of `.unwrap()`s which need to be removed.

My intention will be to make it [Plex](https://plex.tv) for Books.

TODO:
 - Implement database migrations
 - More `TODO`s in comments throughout the code


# Running/Building

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



# Gallery

## Basic library viewer.
![Home](https://i.thick.at/SelfDispleasedNewt513.jpeg)

## Basic book viewer.
![Book View](https://i.thick.at/UnrousedCuran345.png)

## Basic book reader that is implemented currently.
![Book Reader](https://i.thick.at/UnreckonableSparrow115.png)

## Notes for the book viewer. Will hopefully be combined with the book along with a popup(?) instead of being to the side.
![Book Reader With Notes](https://i.thick.at/BrannierShay152.png)

## Options for the book viewer. **Will be somewhere else entirely.**
![Book Reader With Options](https://i.thick.at/AdmissiveFlyingSquirrel582.png)

## It can even go fullscreen!
![Book Reader Fullscreen](https://i.thick.at/WayfarerHastingsPursuivant867.png)