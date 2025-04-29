# Cktool Coomer Downloader

A Rust-based CLI tool for downloading content from coomer.su and kemono.su platforms.

## Features

- Simple and easy-to-use command-line interface
- Download content from any profile using URL
- Support for both coomer.su and kemono.su platforms
- Custom output directory support
- Fast and efficient downloads

## Installation

### Using Cargo (Recommended)

1. First, install Rust by following the instructions at [rust-lang.org](https://www.rust-lang.org/learn/get-started)

2. Install cktool using cargo:
```bash
cargo install cktool
```

or install from github repo.

```bash
cargo install --git=https://github.com/HermesMaker/cktool
```

### Binary Installation

Pre-compiled binaries are available in the [releases](https://github.com/HermesMaker/cktool/releases) section.

You can use `cargo-binstall` to install pre-compiled binaries with command below.

```bash
cargo binstall cktool
```

## Usage

### Basic Usage

Download content from a profile using its URL:

```bash
cktool https://coomer.su/fansly/user/12345
```

Download content only single post.

```bash
cktool https://coomer.su/fansly/user/12345/post/6789
```

### Specifying page download (50 posts)

```bash
cktool https://coomer.su/fansly/user/12345 -p 1

```

### Specifying Output Directory

You can specify a custom output directory for the downloaded content:

```bash
cktool https://coomer.su/fansly/user/12345 --out /path/to/output/directory
```

## Advanced usage

### `-t` or `--task` option

```bash
cktool <url> --task 50
```

With `task` option you can specify the maximum number of posts that can be downloaded at once. Increasing the number can reduce time, but increases the risk of
<b>too many requests errors</b>.

### `-r` or `--retry` option

```bash
cktool <url> --retry 20
```

With `retry` option you can specify the maximum number of re-downloads when found any error. To reduce the chances of failed downloads.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the GNU General Public License v3.0 (GPL-3.0) - see the [LICENSE](LICENSE) file for details.

## Support me                                                                                                                                     
-bitcoin: [14wptNGSb4sVfNnMLnKrb3Vbntd1FYBxyn](bitcoin:14wptNGSb4sVfNnMLnKrb3Vbntd1FYBxyn)
