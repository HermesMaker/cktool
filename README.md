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

### Save failed URLs to file

You can save failed URLs to file with `--log` flag. To re-download with `ckret` command.

When each URL is successfully downloaded, `ckret` will add a `#` in front of URL.
This means that if you encounter failed download, you can run `ckret` command again,
and any links with a `#` in front will be skipped.

```bash
cktool https://coomer.su/fansly/user/12345 --log # default output file is ./failed.log
# or
cktool https://coomer.su/fansly/user/12345 --log fail.txt # custom output file.

```

### `ckret` command

This command works with `failed.log` (Files obtained from the --log flag.) to redownload the failed files.

```bash
ckret failed.log
ckret failed.log --out folder # Save downloaded files to specific folder
ckret failed.log --retry 100 # define retry times when failed.

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

### `-v` or `--video-only` option

```bash
cktool <url> --video-only
```

### `-i` or `--image-only` option

```bash
cktool <url> --image-only
```

### `--verbose` option

```bash
cktool <url> --verbose
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the GNU General Public License v3.0 (GPL-3.0) - see the [LICENSE](LICENSE) file for details.

## Support me

-bitcoin: [12ukxPmuXkyi4QHrxwZgaok2yiD6GrP39A](bitcoin:12ukxPmuXkyi4QHrxwZgaok2yiD6GrP39A)
