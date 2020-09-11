# pipe-rename

pipe-rename takes a list of files as input, opens your \$EDITOR of choice, then
renames those files accordingly.

[![asciicast](https://asciinema.org/a/NdYTt0JeMyZpVZCNLJL1Opq19.svg)](https://asciinema.org/a/NdYTt0JeMyZpVZCNLJL1Opq19)

## Installation

`cargo install pipe-rename`

This will install the `renamer` binary.

## Usage

Usage is simple, just pipe a list of files into `renamer`. This will open your
\$EDITOR (or vim, if not set), and once your editor exits it will detect which
files were renamed.
