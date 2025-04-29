# nix-sweep
`nix-sweep` aims to provide a nice interface for cleaning up old Nix profile generations and left-over garbage collection roots.

## Example: Remove older systems generations, but keep at least 10
```console
$ sudo nix-sweep cleanout --remove-older 20 --keep-min 10 system
=> Listing system generations
[187]    47 days old, would remove      [21.0 GiB / 2.88 MiB]
[188]    42 days old, would remove      [21.0 GiB / 2.88 MiB]
[189]    42 days old, would remove      [21.0 GiB / 3.84 MiB]
[190]    41 days old, would remove      [21.0 GiB / 25.4 MiB]
[191]    38 days old, would remove      [21.0 GiB / 25.4 MiB]
[192]    27 days old, would remove      [20.8 GiB / 183 KiB]
[193]    21 days old, would remove      [20.8 GiB / 183 KiB]
[194]    21 days old, would remove      [20.8 GiB / 183 KiB]
[195]    21 days old, would remove      [20.8 GiB / 183 KiB]
[196]    21 days old, would remove      [20.8 GiB / 183 KiB]
[197]    21 days old, would remove      [20.8 GiB / 183 KiB]
[198]    20 days old, would remove      [20.8 GiB / 182 KiB]
[199]    20 days old, would remove      [20.8 GiB / 183 KiB]
[200]    20 days old, would remove      [20.8 GiB / 0 bytes]
[201]    20 days old, would remove      [20.8 GiB / 182 KiB]
[202]    20 days old, would remove      [20.8 GiB / 183 KiB]
[203]    20 days old, would keep        [20.8 GiB / 0 bytes]
[204]    20 days old, would keep        [20.8 GiB / 1.85 GiB]
[205]    3 days old, would keep         [17.1 GiB / 183 KiB]
[206]    3 days old, would keep         [17.1 GiB / 0 bytes]
[207]    3 days old, would keep         [17.2 GiB / 0 bytes]
[208]    3 days old, would keep         [17.1 GiB / 0 bytes]
[209]    3 days old, would keep         [17.2 GiB / 0 bytes]
[210]    0 days old, would keep         [17.2 GiB / 182 KiB]
[211]    0 days old, would keep         [17.2 GiB / 1.98 MiB]
[212]    0 days old, would keep         [17.2 GiB / 2.30 MiB]

Do you want to delete the marked generations? [y/N] y

=> Removing old system generations
-> Removing generation 187 (47 days old)
removing profile version 187
-> Removing generation 188 (42 days old)
removing profile version 188
-> Removing generation 189 (42 days old)
removing profile version 189
-> Removing generation 190 (41 days old)
removing profile version 190
-> Removing generation 191 (38 days old)
removing profile version 191
-> Removing generation 192 (27 days old)
removing profile version 192
-> Removing generation 193 (21 days old)
removing profile version 193
-> Removing generation 194 (21 days old)
removing profile version 194
-> Removing generation 195 (21 days old)
removing profile version 195
-> Removing generation 196 (21 days old)
removing profile version 196
-> Removing generation 197 (21 days old)
removing profile version 197
-> Removing generation 198 (20 days old)
removing profile version 198
-> Removing generation 199 (20 days old)
removing profile version 199
-> Removing generation 200 (20 days old)
removing profile version 200
-> Removing generation 201 (20 days old)
removing profile version 201
-> Removing generation 202 (20 days old)
removing profile version 202
-> Keeping generation 203 (20 days old)
-> Keeping generation 204 (20 days old)
-> Keeping generation 205 (3 days old)
-> Keeping generation 206 (3 days old)
-> Keeping generation 207 (3 days old)
-> Keeping generation 208 (3 days old)
-> Keeping generation 209 (3 days old)
-> Keeping generation 210 (0 days old)
-> Keeping generation 211 (0 days old)
-> Keeping generation 212 (0 days old)
```

## Example: Listing GC roots
```console
$ nix-sweep gc-roots
/home/jzbor/result [31.3 MiB / 205 KiB]
  -> /nix/store/9bwryidal9q3g91cjm6xschfn4ikd82q-hello-2.12.1

/home/jzbor/result-1 [43.2 MiB / 0 bytes]
  -> /nix/store/h5rn37dd6vfvr9xb0jq85sq8hf6xchry-coreutils-9.6
```

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

You can generate a preset config with the `generate-preset` subcommand:
```console
$ nix-sweep generate-preset -p housekeeping --keep-min 10 --remove-older 14
[housekeeping]
keep-min = 10
remove-older = 14
interactive = true
gc = false
```

Presets can be used with the `-p` (`--preset`) flag:
```console
nix-sweep -p housekeeping system
nix-sweep -p only-remove-really-old system
nix-sweep -p nuke-everything system
```
