# Plan 105 Status: First Eggfetch-Enabled Release and Five-Target Qualification

Status: planned; not yet executed.

## Objective

Publish the first stable StegoEggo CLI release whose shipped binary contains
the native `eggfetch-core` updater, then qualify that exact release through
the real five-target GitHub Actions matrix and public installer path.

## Preconditions

Plans 103-104 are implemented and locally qualified.

The currently published `v0.4.1` artifacts predate the eggfetch migration and
must not be used as proof that the new updater dependency graph builds on
Windows or ships correctly.

## Required closeout evidence

Record before marking complete:

- selected next unused synchronized workspace version;
- release source commit SHA;
- successful carrier -> library -> CLI crates.io publication;
- exact Git tag and GitHub Release;
- successful `release-binaries.yml` run for all five targets;
- explicit Windows x86_64 job success on the native Windows runner;
- Linux x86_64 and aarch64 GLIBC_2.17 floor evidence;
- complete public asset audit;
- public Unix installer smoke;
- released binary `stegoeggo update` current-version smoke using eggfetch;
- confirmation that installed updater operation does not require curl;
- correction of the stale “before any download” preflight wording.

## Deferred by design

A real public eggfetch-updater replacement from one native-updater release to a
newer native-updater release is Plan 106. Plan 105 may close after the first
eggfetch-enabled stable release is fully qualified even if no second such
release exists yet.
