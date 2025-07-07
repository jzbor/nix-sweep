# nix-sweep
`nix-sweep` aims to provide a nice interface for cleaning up old Nix profile generations and left-over garbage collection roots.

![nix-sweep demo](https://files.jzbor.de/github/nix-sweep-demo.gif)


## Size Estimates
In addition to information like the age of a generation or the path of a gc root `nix-sweep` also displays the size of a generation or gc root (e.g.`[20.8 GiB / 1.85 GiB]`).

The first number displayed is the full size of the closure (all Nix paths that are required by a generation or gc root).
The second number shows the size of all dependencies that are not also used by another member of the group.
This may give a rough lower bound of how much space would be freed if that particular generation or gc root were to be discarded.

Calculating the size of the Nix paths may take a few moments, especially on older hardware.
If you want to avoid that overhead you can use `--no-size` to skip size calculations.

## Presets
`nix-sweep` allows you to create presets for clean out criteria, that can then be used with `nix-sweep cleanout`.

Preset configs are stored as [TOML](https://toml.io) files.
If a preset is present in multiple of those files, then the ones further down in the list override ones further up.
The following locations are checked for preset files:
* `/etc/nix-sweep/presets.toml`
* `$XDG_CONFIG_HOME/nix-sweep/presets.toml`/`~/.config/nix-sweep/presets.toml`
* configuration files passed via `-C`/`--config`

Example:
```yaml
[housekeeping]
keep-min = 10
remove-older = 14d
interactive = true
gc = false
```

Presets can be used with the `-p` (`--preset`) flag:
```console
nix-sweep -p housekeeping system
nix-sweep -p only-remove-really-old system
nix-sweep -p nuke-everything system
```

## Contributing
Code contributions (pull request) are **currently not accepted**.
If you have any feedback, ideas or bugreports feel free to open a [new issue](https://github.com/jzbor/nix-sweep/issues/new)
