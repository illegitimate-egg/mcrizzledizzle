alias r := run
alias t := test
alias b := book

run:
  cd rte; cargo run

test:
  cargo test

book:
  cd book; mdbook serve

install_book_toolchain:
  cargo install mdbook --locked --version 0.4.47
  # cargo install --locked --path packages/mdbook-trpl # mdbook repo needed for this one lmao
