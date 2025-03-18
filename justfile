alias r := run
alias t := test
alias b := build

run:
  cd rte; cargo run

test:
  cargo test

build:
  mdbook build

install_book_toolchain:
  cargo install mdbook --locked --version 0.4.47
  # cargo install --locked --path packages/mdbook-trpl # mdbook repo needed for this one lmao
