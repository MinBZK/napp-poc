# Deze repository is gearchiveerd; wat er nog draait is de 301 naar de poc op
# zijn nieuwe plek. De recepten voor de applicatie (wetten valideren, BDD,
# WASM, dev-server, docker) staan in de monorepo, MinBZK/regelrecht.

# List available commands
default:
    @just --list

# Run the redirect's tests
test:
    cargo test -p napp-redirect

# Run the redirect locally (PORT overrides the 8400 default)
run:
    cargo run -p napp-redirect

# Build the image that the deploy publishes
docker-build:
    docker build -t napp-redirect .
