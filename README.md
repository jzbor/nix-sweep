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
