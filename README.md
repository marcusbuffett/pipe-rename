# pipe-rename
<!-- ALL-CONTRIBUTORS-BADGE:START - Do not remove or modify this section -->
[![All Contributors](https://img.shields.io/badge/all_contributors-1-orange.svg?style=flat-square)](#contributors-)
<!-- ALL-CONTRIBUTORS-BADGE:END -->

`pipe-rename` takes a list of files as input, opens your \$EDITOR of choice, then
renames those files accordingly.

[![asciicast](https://asciinema.org/a/NdYTt0JeMyZpVZCNLJL1Opq19.svg)](https://asciinema.org/a/NdYTt0JeMyZpVZCNLJL1Opq19)

## Installation

`cargo install pipe-rename`

This will install the `renamer` binary.

## Usage

Usage is simple, just pipe a list of files into `renamer`. This will open your
\$EDITOR (or vim, if not set), and once your editor exits it will detect which
files were renamed.

Helptext:

```
Takes a list of files and renames/removes them, by piping them through an external editor

USAGE:
    renamer [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -c, --rename-command <COMMAND>    Optionally set a custom rename command, like 'git mv'
```

## Contributors ✨

Thanks goes to these wonderful people ([emoji key](https://allcontributors.org/docs/en/emoji-key)):

<!-- ALL-CONTRIBUTORS-LIST:START - Do not remove or modify this section -->
<!-- prettier-ignore-start -->
<!-- markdownlint-disable -->
<table>
  <tr>
    <td align="center"><a href="https://mbuffett.com/"><img src="https://avatars3.githubusercontent.com/u/1834328?v=4?s=100" width="100px;" alt=""/><br /><sub><b>Marcus Buffett</b></sub></a><br /><a href="#ideas-marcusbuffett" title="Ideas, Planning, & Feedback">🤔</a> <a href="https://github.com/marcusbuffett/pipe-rename/commits?author=marcusbuffett" title="Code">💻</a> <a href="#design-marcusbuffett" title="Design">🎨</a></td>
  </tr>
</table>

<!-- markdownlint-restore -->
<!-- prettier-ignore-end -->
<!-- ALL-CONTRIBUTORS-LIST:END -->

This project follows the [all-contributors](https://github.com/all-contributors/all-contributors) specification. Contributions of any kind welcome!