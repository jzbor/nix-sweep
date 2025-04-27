This project aims to provide a nice interface for cleaning up Nix profiles and left-over garbage collection roots.

## Example for `cleanout`:
```console
$ nix-sweep cleanout --keep-max 2 system home user
=> Listing system generations
[279]    12 days old, would remove
[280]    2 days old, would keep
[281]    1 days old, would keep

Do you want to proceed? [y/N]
-> Not touching profile

=> Listing home-manager generations
[708]    2 days old, would remove
[709]    2 days old, would remove
[710]    2 days old, would remove
[711]    2 days old, would remove
[712]    2 days old, would remove
[713]    2 days old, would remove
[714]    2 days old, would keep
[715]    1 days old, would keep

Do you want to proceed? [y/N]
-> Not touching profile

=> Listing user profile generations
[1399]   2 days old, would remove
[1400]   2 days old, would remove
[1401]   2 days old, would remove
[1402]   2 days old, would remove
[1403]   2 days old, would remove
[1404]   2 days old, would remove
[1405]   2 days old, would remove
[1406]   1 days old, would remove
[1407]   1 days old, would keep
[1408]   1 days old, would keep

Do you want to proceed? [y/N] y

=> Removing old user profile generations
-> Removing generation 1399 (2 days old)
removing profile version 1399
-> Removing generation 1400 (2 days old)
removing profile version 1400
-> Removing generation 1401 (2 days old)
removing profile version 1401
-> Removing generation 1402 (2 days old)
removing profile version 1402
-> Removing generation 1403 (2 days old)
removing profile version 1403
-> Removing generation 1404 (2 days old)
removing profile version 1404
-> Removing generation 1405 (2 days old)
removing profile version 1405
-> Removing generation 1406 (1 day old)
removing profile version 1406
-> Keeping generation 1407 (1 day old)
-> Keeping generation 1408 (1 day old)
```

## Example for `gc-roots`:
```console
$ nix-sweep gc-roots
/home/user/result [52.0 MiB]
  -> /nix/store/9bwryidal9q3g91cjm6xschfn4ikd82q-hello-2.12.1

/home/user/result-1 [237 MiB]
  -> /nix/store/h5rn37dd6vfvr9xb0jq85sq8hf6xchry-coreutils-9.6
```
