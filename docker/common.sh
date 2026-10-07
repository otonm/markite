# Shared by the docker/*.sh scripts: `. "$(dirname "$0")/common.sh"` after `set -eu`. Changes to the repo root.
cd "$(dirname "$0")/.."

# Runs a command in a container with the repo mounted at /src.
run() { sudo docker run --rm -v "$PWD":/src -w /src "$@"; }

# Containers run as root: hand the outputs back to the user even when a step fails.
trap 'sudo chown -R "$(id -u):$(id -g)" builds .docker-cache Cargo.lock 2>/dev/null || true' EXIT
