# lzfoo

A fast, memory efficient and stream capable [lzfse](https://github.com/lzfse/lzfse) command line tool clone.
Powered by [manic_lzfse](https://github.com/shampoofactory/manic_lzfse).


```
$ lzfoo
lzfoo 0.1.0
Vin Singh <github.com/shampoofactory>
LZFSE compressor/ decompressor

USAGE:
    lzfoo <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    -decode    Decode (decompress)
    -encode    Encode (compress)
    help       Prints this message or the help of the given subcommand(s)

See 'lzfoo help <command>' for more information on a specific command.
```

## Installation


`lzfoo` is on crates.io:

```
$ cargo install lzfoo
```

## Basic usage

```
$ lzfoo help -encode
lzfoo--encode 
Encode (compress)

USAGE:
    lzfoo -encode [FLAGS] [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -v               Sets the level of verbosity
    -V, --version    Prints version information

OPTIONS:
    -i <FILE>        input
    -o <FILE>        output

If no input/ output specified reads/ writes from standard input/ output
```

Compress `a.txt` to `a.txt.lzfse`:
```
$ lzfoo -encode -i a.txt -o a.txt.lzfse
```

Compress with stdin/ stdout:
```
$ lzfoo -encode -i < a.txt > a.txt.lzfse
```
```
$ echo "semper fidelis" | lzfoo -encode > a.txt.lzfse
```

```
$ lzfoo help -decode
lzfoo--decode 
Decode (decompress)

USAGE:
    lzfoo -decode [FLAGS] [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -v               Sets the level of verbosity
    -V, --version    Prints version information

OPTIONS:
    -i <FILE>        input
    -o <FILE>        output

If no input/ output specified reads/ writes from standard input/ output.
```

Decompress `a.txt.lzfse` to `a.txt`:
```
$ lzfoo -decode -i a.txt.lzfse -o a.txt
```

Decompress with stdin/ stdout:
```
$ lzfoo -decode -i < a.txt.lzfse > a.txt
```
```
$ cat a.txt.lzfse | lzfoo -decode
```

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.