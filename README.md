This project aims to provide a nicer interface for cleaning up Nix profiles:
```console
$ nix-housekeeping --keep 20 --older 20 --list
=> Listing profile generations for user jzbor
[1288]  age: 29d        marked: remove
[1289]  age: 29d        marked: remove
[1290]  age: 29d        marked: remove
[1291]  age: 29d        marked: remove
[1292]  age: 29d        marked: remove
[1293]  age: 29d        marked: remove
[1294]  age: 29d        marked: remove
[1295]  age: 29d        marked: remove
[1296]  age: 29d        marked: remove
[1297]  age: 29d        marked: remove
[1298]  age: 29d        marked: remove
[1299]  age: 29d        marked: remove
[1300]  age: 29d        marked: remove
[1301]  age: 29d        marked: remove
[1302]  age: 29d        marked: remove
[1303]  age: 29d        marked: remove
[1304]  age: 29d        marked: remove
[1305]  age: 29d        marked: remove
[1306]  age: 27d        marked: remove
[1307]  age: 27d        marked: remove
[1308]  age: 27d        marked: keep
[1309]  age: 27d        marked: keep
[1310]  age: 19d        marked: keep
[1311]  age: 19d        marked: keep
[1312]  age: 18d        marked: keep
[1313]  age: 18d        marked: keep
[1314]  age: 18d        marked: keep
[1315]  age: 18d        marked: keep
[1316]  age: 18d        marked: keep
[1317]  age: 18d        marked: keep
[1318]  age: 18d        marked: keep
[1319]  age: 18d        marked: keep
[1320]  age: 18d        marked: keep
[1321]  age: 18d        marked: keep
[1322]  age: 18d        marked: keep
[1323]  age: 18d        marked: keep
[1324]  age: 12d        marked: keep
[1325]  age: 12d        marked: keep
[1326]  age: 12d        marked: keep
[1327]  age: 12d        marked: keep

$ nix-housekeeping --keep 20 --older 20
=> Removing old profile generations for user jzbor
-> Removing generation 1288 (29 days old)
removing profile version 1288
-> Removing generation 1289 (29 days old)
removing profile version 1289
-> Removing generation 1290 (29 days old)
removing profile version 1290
-> Removing generation 1291 (29 days old)
removing profile version 1291
-> Removing generation 1292 (29 days old)
removing profile version 1292
-> Removing generation 1293 (29 days old)
removing profile version 1293
-> Removing generation 1294 (29 days old)
removing profile version 1294
-> Removing generation 1295 (29 days old)
removing profile version 1295
-> Removing generation 1296 (29 days old)
removing profile version 1296
-> Removing generation 1297 (29 days old)
removing profile version 1297
-> Removing generation 1298 (29 days old)
removing profile version 1298
-> Removing generation 1299 (29 days old)
removing profile version 1299
-> Removing generation 1300 (29 days old)
removing profile version 1300
-> Removing generation 1301 (29 days old)
removing profile version 1301
-> Removing generation 1302 (29 days old)
removing profile version 1302
-> Removing generation 1303 (29 days old)
removing profile version 1303
-> Removing generation 1304 (29 days old)
removing profile version 1304
-> Removing generation 1305 (29 days old)
removing profile version 1305
-> Removing generation 1306 (27 days old)
removing profile version 1306
-> Removing generation 1307 (27 days old)
removing profile version 1307
-> Keeping generation 1308 (27 days old)
-> Keeping generation 1309 (27 days old)
-> Keeping generation 1310 (19 days old)
-> Keeping generation 1311 (19 days old)
-> Keeping generation 1312 (18 days old)
-> Keeping generation 1313 (18 days old)
-> Keeping generation 1314 (18 days old)
-> Keeping generation 1315 (18 days old)
-> Keeping generation 1316 (18 days old)
-> Keeping generation 1317 (18 days old)
-> Keeping generation 1318 (18 days old)
-> Keeping generation 1319 (18 days old)
-> Keeping generation 1320 (18 days old)
-> Keeping generation 1321 (18 days old)
-> Keeping generation 1322 (18 days old)
-> Keeping generation 1323 (18 days old)
-> Keeping generation 1324 (12 days old)
-> Keeping generation 1325 (12 days old)
-> Keeping generation 1326 (12 days old)
-> Keeping generation 1327 (12 days old)
```
