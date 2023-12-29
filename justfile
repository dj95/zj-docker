bench:
  cargo wasi bench --features bench -- --color always | grep --color=never -v "Criterion.rs ERROR"

run: build
    zellij -l ./plugin-dev-workspace.kdl -s zj-docker-compose-dev

build:
  cargo build

test:
  cargo wasi test -- --nocapture

lint:
  cargo clippy --all-targets -- -D warnings
  cargo audit

release version:
  cargo set-version {{version}}
  direnv exec . cargo build --release
  git commit -am "chore: bump version to v{{version}}"
  git tag -m "v{{version}}" v{{version}}
  git cliff --current
