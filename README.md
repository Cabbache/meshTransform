# meshTransform
rust tool that transforms ascii .obj or .stl files from stdin

![Warp](https://Cabbache.github.io/cow.gif)

### Usage

```console
Usage: mesh_transform <COMMAND>

Commands:
  translate  Translates object
  rotate     Rotates object
  scale      Scales object
  warp       Warps object. This transformation is non-linear
  help       Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

### Example
`mesh_transform translate 1.2 -0.3 -9 < example/cow.stl | mesh_transform scale 1 1 2 > transformed.stl`
