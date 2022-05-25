# pipe-rename

[![Crates.io](https://img.shields.io/crates/v/pipe-rename)](https://crates.io/crates/pipe-rename)

`pipe-rename` takes a list of files as input, opens your \$EDITOR of choice, then
renames those files accordingly.

![](renamer.gif)

## Installation

`cargo install pipe-rename`

This will install the `renamer` binary.

## Usage

Usage is simple, just pipe a list of files into `renamer`. This will open your
\$EDITOR (or vim, if not set), and once your editor exits it will detect which
files were renamed:

```bash
ls | renamer
```

You can also supply filenames as positional arguments. To rename txt files in the current directory:

```bash
renamer *.txt
```

The default behavior is to rename files, but you can override this. If you want
to run `git mv old new` on each rename, you can do something like this:

```bash
ls | renamer --rename-command "git mv"
```

## Helptext

```
Takes a list of files and renames/moves them by piping them through an external editor

USAGE:
    renamer [OPTIONS] [FILES]...

ARGS:
    <FILES>...

OPTIONS:
    -c, --rename-command <RENAME_COMMAND>    Optionally set a custom rename command, like 'git mv'
    -f, --force                              Overwrite existing files
    -h, --help                               Print help information
    -p, --pretty-diff                        Prettify diffs
    -V, --version                            Print version information
    -y, --yes                                Answer all prompts with yes
```

## Contributors ✨

<!-- ALL-CONTRIBUTORS-LIST:START - Do not remove or modify this section -->
<!-- prettier-ignore-start -->
<!-- markdownlint-disable -->
<table>
  <tr>
    <td align="center"><a href="https://mbuffett.com/"><img src="https://avatars3.githubusercontent.com/u/1834328?v=4?s=100" width="100px;" alt=""/><br /><sub><b>Marcus Buffett</b></sub></a><br /><a href="#ideas-marcusbuffett" title="Ideas, Planning, & Feedback">🤔</a> <a href="https://github.com/marcusbuffett/pipe-rename/commits?author=marcusbuffett" title="Code">💻</a></td>
    <td align="center"><a href="https://git.ireas.org/"><img src="https://avatars2.githubusercontent.com/u/165115?v=4?s=100" width="100px;" alt=""/><br /><sub><b>Robin Krahl</b></sub></a><br /><a href="#ideas-robinkrahl" title="Ideas, Planning, & Feedback">🤔</a> <a href="https://github.com/marcusbuffett/pipe-rename/commits?author=robinkrahl" title="Code">💻</a> <a href="https://github.com/marcusbuffett/pipe-rename/issues?q=author%3Arobinkrahl" title="Bug reports">🐛</a></td>
    <td align="center"><a href="https://timkovi.ch/"><img src="https://avatars.githubusercontent.com/u/651077?v=4?s=100" width="100px;" alt=""/><br /><sub><b>Max Timkovich</b></sub></a><br /><a href="https://github.com/marcusbuffett/pipe-rename/commits?author=mtimkovich" title="Code">💻</a></td>
    <td align="center"><a href="https://github.com/bew"><img src="https://avatars.githubusercontent.com/u/9730330?v=4?s=100" width="100px;" alt=""/><br /><sub><b>Benoit de Chezelles</b></sub></a><br /><a href="#ideas-bew" title="Ideas, Planning, & Feedback">🤔</a></td>
  </tr>
</table>

<!-- markdownlint-restore -->
<!-- prettier-ignore-end -->

<!-- ALL-CONTRIBUTORS-LIST:END -->

This project follows the [all-contributors](https://github.com/all-contributors/all-contributors) specification. Contributions of any kind welcome!
