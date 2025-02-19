This project aims to provide a nicer interface for cleaning up Nix profiles:
```console
$ nix-sweep --user --home --older 7
=> Listing home-manager generations
[311]    11 days old, would remove
[312]    10 days old, would remove
[313]    10 days old, would remove
[314]    10 days old, would remove
[315]    10 days old, would remove
[316]    10 days old, would remove
[317]    10 days old, would remove
[318]    10 days old, would keep
[319]    7 days old, would keep
[320]    7 days old, would keep
[321]    7 days old, would keep
[322]    7 days old, would keep
[323]    7 days old, would keep
[324]    7 days old, would keep
[325]    5 days old, would keep
[326]    4 days old, would keep
[327]    2 days old, would keep

Do you want to proceed? [y/n] y

=> Removing old home-manager generations
-> Removing generation 311 (11 days old)
removing profile version 311
-> Removing generation 312 (10 days old)
removing profile version 312
-> Removing generation 313 (10 days old)
removing profile version 313
-> Removing generation 314 (10 days old)
removing profile version 314
-> Removing generation 315 (10 days old)
removing profile version 315
-> Removing generation 316 (10 days old)
removing profile version 316
-> Removing generation 317 (10 days old)
removing profile version 317
-> Keeping generation 318 (10 days old)
-> Keeping generation 319 (7 days old)
-> Keeping generation 320 (7 days old)
-> Keeping generation 321 (7 days old)
-> Keeping generation 322 (7 days old)
-> Keeping generation 323 (7 days old)
-> Keeping generation 324 (7 days old)
-> Keeping generation 325 (5 days old)
-> Keeping generation 326 (4 days old)
-> Keeping generation 327 (2 days old)

=> Listing user profile generations
[1126]   4 days old, would keep
[1127]   2 days old, would keep
[1128]   2 days old, would keep
[1129]   2 days old, would keep
[1130]   0 days old, would keep
[1131]   0 days old, would keep
[1132]   0 days old, would keep
[1133]   0 days old, would keep

Do you want to proceed? [y/n] y

=> Removing old user profile generations
-> Keeping generation 1126 (4 days old)
-> Keeping generation 1127 (2 days old)
-> Keeping generation 1128 (2 days old)
-> Keeping generation 1129 (2 days old)
-> Keeping generation 1130 (0 days old)
-> Keeping generation 1131 (0 days old)
-> Keeping generation 1132 (0 days old)
-> Keeping generation 1133 (0 days old)


=> Running garbage collection
finding garbage collector roots...
removing stale link from '/nix/var/nix/gcroots/auto/sy8ys56k6ip6fc5j98dl76a02awwk4gc' to '/home/jzbor/.local/state/nix/profiles/home-manager-317-link'
removing stale link from '/nix/var/nix/gcroots/auto/hn10hm7spbxxhxac2ynag64g6739ai67' to '/home/jzbor/.local/state/nix/profiles/home-manager-316-link'
removing stale link from '/nix/var/nix/gcroots/auto/rqir1y11d5ijx97518bkvvnpbmh4ga9y' to '/home/jzbor/.local/state/nix/profiles/home-manager-311-link'
removing stale link from '/nix/var/nix/gcroots/auto/cbj8xfglwv4nlgwsnbx9ma294b64hmzc' to '/home/jzbor/.local/state/nix/profiles/home-manager-314-link'
removing stale link from '/nix/var/nix/gcroots/auto/cyab8rhkkb0v7ffgcypimwbi27azskx7' to '/home/jzbor/.local/state/nix/profiles/home-manager-312-link'
removing stale link from '/nix/var/nix/gcroots/auto/g6cir9677jwz2sz36mhaw9w4n9h8d2zi' to '/home/jzbor/.local/state/nix/profiles/home-manager-313-link'
...
```
