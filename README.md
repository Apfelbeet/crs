# CRS

Concurrent Regression Search (CRS) is a tool for finding software regression in version control systems.

## Usage

```sh
crs <REPOSITORY> <TEST> [OPTIONS] --start <START>  [-- <TARGETS>...]
```

|Arguments | Description
| --- | --- |
| Repository | Path to the root directory of your repository. |
| Test | Path to a script, that evaluates if a version is valid or not. It will be executed in the root directory of the responsible worktree. |

| Option/Flag | short | Description | Mandatory | Default
| --- | --- | --- | --- | --- |
|--start | -s | Commit hash of the root. | yes | - |
|--processes | -p | Amount of threads that can be spawned. | no | 1 |
|--worktree-location |  | By default *crs* will spawn all worktrees in a subdirectory of the source repository. You can change that location by specifying another path here.  | no |  |
|--search-mode |   | *crs* implements multiple search modes. List of supported search modes: rpa-binary, rpa-linear, rpa-multi | no | rpa-binary |
|--interrupt| | *crs* will kill running processes, when they are no longer relevant. This might speed up the search, but is only possible if the test script can be interrupted without additional clean up steps. | no | false |
|--log| -l | Takes a path as an argument. Creates directory and writes files with all stdout and stderr output of the processes and a summary of all queries (time, result, ...) | no | |
|--no-propagate |   | Disables propagation of regression points.  | no | false |
|--no-extended |   | Disables extended RPA implementation. Extended RPA finds regression points, that are more likely to be related to the problem, but takes additional validity queries. | no | false |

The default configuration would look like:

```sh
crs <path to repository> <path to script> \
 -p <amount threads> \
 -s <commit hash of root> \
 -- <commit hash of 1st target> [<commit hash of 2nd target> [...]]
```

### Test Script

The test script will be executed whenever *crs* queries a commit. For that the
script is executed in the root directory of the responsible worktree for that
commit. The exit code determines the result for the test. Exit with 0: commit is
valid, 1-127 (without 125) is invalid, 125 is untestable (commit will be
ignored) and any other value will stop *crs*. Don't forget to specify the
interpreter in the first line.

So you script might follow this structure:

```sh
#!/bin/sh

# your tests

exit <code>
```

### Example

For example, say we are in the root directory of the repository and have a test
`crs_test.sh` in the same directory. And we know that the the 2 week old commit
`fff777fff777fff777fff777fff777fff777fff7` evaluates to true and the two new commits `eee666eee666eee666eee666eee666eee666eee6`,
`ddd555ddd555ddd555ddd555ddd555ddd555ddd5` from branch A and B evaluate to false. With 8 cores on your machine you can run:

```sh
crs ./ ./crs_test.sh -p 8 -s fff7…ff7 -- eee6…ee6 ddd5…dd5
```
