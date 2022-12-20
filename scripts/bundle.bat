cd %~dp0..\crates\frontend

trunk build --release --public-url "/dist" -d "../../app/public/dist"

cd %~dp0..

cargo build --bin backend-bundled --release --features=bundled

echo Backend Bundle Built in target/release