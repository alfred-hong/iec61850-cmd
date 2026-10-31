# iec61850-cmd

Pure Rust IEC 61850 client and server. The library crate is `iec61850-cmd` (`crates/iec61850`). The client command is `iec61850_client` (package `iec61850-cmd-client`). The server command is `iec61850_server` (package `iec61850-cmd-server`). Neither calls libiec61850.

Acceptance is interoperation with the libiec61850 demo device, not a claim of full-standard conformance. The library owns the messages. The command line owns flags, output, exit codes, and the guards.

## Build

```sh
cargo build -p iec61850-cmd-client -p iec61850-cmd-server
cargo run -p iec61850-cmd-client --bin iec61850_client -- version
cargo run -p iec61850-cmd-server --bin iec61850_server -- version
```

`iec61850_client version` prints `iec61850_client 0.0.1` and exits 0. `iec61850_server version` prints `iec61850_server 0.0.1` and exits 0. `--help` also exits 0.

## What runs today

Client `version` prints the package version.

These client commands open one association, use the model the library loads, and send MMS Conclude before exit. stderr starts with one `action=<command>` line. `set object --verify` adds `action=read` before the read-back. Both then print `action=release`. A refused or dropped connection is exit 3. A connect timeout or a confirmed-request timeout is exit 4. These commands were compared with `iec61850ctl` serving that project's `e2e/testdata/interop.icd`. This tree has no libiec61850 server binary, so that peer has not been used yet.

- `list lds` and `list lns` print one sorted name per line. `list dos` prints the data-object name only. `list das` prints `FC name type`. The type is the short name from GetVariableAccessAttributes, such as `bool`, `bit_string`, `utc_time`, or `visible_string`. `--do` accepts the bare name or `FC.DO`. `list dss` prints `LD/LN.dataset`. `list reports` prints `LD/LN.BR.name` or `LD/LN.RP.name`. `list journals` prints `LD/name` when the peer returns names. A Confirmed-Error on that name list is exit 6, not an empty success. The interop device answers VMD-state 4 because it has no journal service. A named logical device, node, or data object that is not in the model is exit 5 with empty stdout. `find path` prints `LD/LN.DO.DA FC`, or exit 5 when nothing matches. A read that the device answers as object-non-existent (MMS data-access error 10, or access-class service error 2) is also exit 5. Other data-access errors, and a Confirmed-Error or Reject that is not object-non-existent, are exit 6.
- `get object`, `get ln`, and `get ds` print CSV with the header `reference,fc,value,type,quality,time`. Boolean values are `0` or `1`. The type column is the MMS name, such as `BOOLEAN`, `INTEGER`, `FLOATING_POINT`, `BIT_STRING`, or `UTC_TIME`. An all-zero quality is `good:0000`. A missing `q` or `t` is `-`. Time is UTC RFC 3339. `stVal` and `Op.general` rows, including data-set members, also fill `quality` and `time` from the sibling `q` and `t`. A logical device missing from this association's domain list uses `type=NOT_ON_DEVICE` and exit 5. A missing node or data set uses `type=NOT_FOUND` and exit 5. `get ds` reads with `specificationWithResult=true` (8-1 Table 45). The response access specification is the dataset name; member order still comes from the data-set directory.
- Model load accepts a buffered report control block in the Edition 1 shape (13 elements, through `TimeOfEntry`, no `ResvTms`) or Edition 2 (14 elements, or 15 with a trailing `Owner` that is dropped). An unbuffered block is 11 elements, or 12 with `Owner` dropped. Without the 13-element shape, a device that only has that block fails the association, including `list lds`. The session user-requirements parameter is the ISO 8327 16-bit value in network byte order. The duplex unit this client requests is the octets `00 02`.
- `set object` writes a scalar. stderr is one `action=write`. `--verify` then prints `action=read` and reads the point back. A mismatch is exit 7 and stderr contains `读回不一致`.

`control operate` without `--yes` prints `存在控制，但是该控制未生效，需要添加确认标识 --yes` and then reads the current status. An object written as `LD/LN.DO` is read as `LD/LN.DO.stVal` with `FC=ST`, in the same CSV as `get object`. The exit code is that read's code. A missing host, a bad reference, or `--format` is still exit 2, and the notice is still printed. `control operate` with `--yes` reads `ctlModel`. Integer 1 (`direct-with-normal-security`) writes `Oper` (`ctlVal`, origin `remote-control`, `ctlNum`, `T`, `Test=false`, `Check` clear) and exits 0. stderr is `action=operate` plus the object and the value. Close is `HV_3_AH_04CTRL/Q0GAPC10.SPCSO` and trip is `HV_3_AH_04CTRL/Q0GAPC11.SPCSO`; each operate changes only that object's `stVal`. A reference ending in `.Oper.ctlVal` is the same object. `status-only` is exit 6 and prints `status-only，对象不可控`. Other ctlModels, `control inspect`, `subscribe report`, and `get journal` still print `尚未实现` with exit 1. The library has no report-subscribe loop, no Select or SelectWithValue state machine, and no ReadJournal. GOOSE and SV list, sniff, subscribe, and publish are the same: there is no SCL reader and no raw Ethernet socket yet. `--yes` is only for `control operate`. Other commands do not take it. Publish still checks `--test` / `--simulation` against `--live` first, so a bad flag is exit 2 and does not send a frame.

`goose sniff`, `goose subscribe`, `goose gocb`, `sv list`, `sv sniff`, `sv subscribe`, and `sv svcb` are not in the client binary yet. Clap reports those names as usage errors (exit 2).

Server `version` prints the package version. Server `start` requires `--scl`. With `--scl`, it parses the flags from PLAN_server.md and then prints `尚未实现` with exit 1. It does not load SCL or listen yet. Step 5 in PLAN_server.md waits until client Step 3 and Step 4 are done.

## Commands

```text
iec61850_client
├── list lds
├── list lns|dos|das|dss|reports|journals
├── tree
├── find path --ln <node> --path <data>
├── get object|ln|ds|journal
├── set object --object <ref> --fc <fc> --value <v> --type <t> [--verify]
├── subscribe report --ld <ld> --ln <ln> --report <name> [--type RP|BR] [--entry-id <16 hex>]
├── control inspect --object <ref>
├── control operate --object <ref> --value <v> --type <t> [--yes]
├── goose list --scl <file>
├── goose publish --iface <nic> --scl <file> --gocb <name> [--test] [--live]
├── sv publish --iface <nic> --scl <file> --svcb <name> [--simulation] [--live]
└── version

iec61850_server
├── start --scl <file> [--ied-name] [--values] [--host] [--port] [--max-connections] [--ready-json]
└── version
```

Device commands need `--host` or `IEC61850_HOST`. `goose list` and `goose` / `sv` `publish` do not. `--ied-name` is parsed and is not sent in MMS Initiate. `list lds` does not filter on it yet.

`set object` and `control operate` take `--type` `bool`, `int`, `uint`, `float`, `enum`, or `string` (case-insensitive). `boolean`, `integer`, `unsigned`, and `double` are accepted as `bool`, `int`, `uint`, and `float`.

`--yes` belongs only to `control operate`. Without it the control is not sent: stderr prints the notice in Error text, and the command reads the status value instead. With `--yes`, `direct-with-normal-security` sends Operate. List, get, set, reports, and GOOSE/SV do not accept `--yes`. Listing and reading only fetch parameters. `--test` and `--simulation` default to true. `--test=false` or `--simulation=false` also requires `--live`. `--live` alone does not clear the bit. A publish that passes these checks still does not send a frame.

## Examples

`192.0.2.10` and the `HV_3_AH_04*` names only show how to write a command. Global flags may be placed before the subcommand on any client command. The cases below name every flag the binaries accept.

Client globals. `--timeout 10` is the same as `10s`. `--format` is `text`, `json`, or `jsonl`.

```sh
iec61850_client \
  --host 192.0.2.10 \
  --port 102 \
  --timeout 10s \
  --connect-timeout 500ms \
  --ied-name HV_3_AH_04 \
  --format json \
  list lds
```

```sh
iec61850_client --host 192.0.2.10 --format text tree
iec61850_client --host 192.0.2.10 --format jsonl tree
```

```sh
iec61850_client version
iec61850_server version
```

List, tree, and find:

```sh
iec61850_client --host 192.0.2.10 list lds
iec61850_client --host 192.0.2.10 list lns --ld HV_3_AH_04CTRL
iec61850_client --host 192.0.2.10 list dos --ld HV_3_AH_04CTRL --ln BIGGIO1
iec61850_client --host 192.0.2.10 list das --ld HV_3_AH_04CTRL --ln BIGGIO1 --do SPCSO10
iec61850_client --host 192.0.2.10 list dss --ld HV_3_AH_04CTRL --ln LLN0
iec61850_client --host 192.0.2.10 list reports --ld HV_3_AH_04CTRL --ln LLN0
iec61850_client --host 192.0.2.10 list journals
iec61850_client --host 192.0.2.10 list journals --ld HV_3_AH_04CTRL
iec61850_client --host 192.0.2.10 tree
iec61850_client --host 192.0.2.10 find path --ln BIGG --path SPCSO10.stVal
```

Read. `--object` with `--fc`, or `--csv`. Not both.

```sh
iec61850_client --host 192.0.2.10 get object \
  --object 'HV_3_AH_04CTRL/BIGGIO1.SPCSO10.stVal' \
  --fc ST

iec61850_client --host 192.0.2.10 get object --csv points.csv

iec61850_client --host 192.0.2.10 get ln --ln HV_3_AH_04CTRL/BIGGIO1
iec61850_client --host 192.0.2.10 get ln --ln HV_3_AH_04CTRL/BIGGIO1 --fc ST

iec61850_client --host 192.0.2.10 get ds --dataset HV_3_AH_04LD0/LLN0.dsGoose

iec61850_client --host 192.0.2.10 get journal \
  --ld HV_3_AH_04CTRL Log \
  --from 2026-10-29T00:00:00Z \
  --to 2026-10-29T23:59:59Z \
  --entry 0001020304050607
```

`points.csv` is the `--csv` input. `reference` and `fc` are required. Any other column is copied through in the same order. Do not use `value`, `type`, `quality`, or `time`; the client appends those.

```csv
name,reference,fc
SPCSO10,HV_3_AH_04CTRL/BIGGIO1.SPCSO10.stVal,ST
general,HV_3_AH_04PROT/OvlPTOC1.Op.general,ST
```

`get object --csv points.csv` prints that table with the live columns last. A single `get object`, `get ln`, or `get ds` uses the same live columns and no extra input columns. Boolean values are `0` or `1`. An all-zero quality is `good:0000`. Time is UTC RFC 3339. A missing `q` or `t` is `-`.

```csv
name,reference,fc,value,type,quality,time
SPCSO10,HV_3_AH_04CTRL/BIGGIO1.SPCSO10.stVal,ST,0,BOOLEAN,good:0000,2026-10-29T02:23:01.123Z
general,HV_3_AH_04PROT/OvlPTOC1.Op.general,ST,0,BOOLEAN,good:0000,2026-10-29T02:23:01.123Z
```

```csv
reference,fc,value,type,quality,time
HV_3_AH_04CTRL/BIGGIO1.SPCSO10.stVal,ST,0,BOOLEAN,good:0000,2026-10-29T02:23:01.123Z
```

`get journal` is not wired yet. When it is, stdout is this CSV and does not take `--format`:

```csv
entry_id,time,tag,value
0001020304050607,2026-10-29T02:23:01.123Z,HV_3_AH_04CTRL/BIGGIO1$ST$SPCSO10$stVal,0
```

Write and control. `--type` here is `bool`. `--verify` reads the point back.

```sh
iec61850_client --host 192.0.2.10 set object \
  --object 'HV_3_AH_04CTRL/BIGGIO1.SPCSO10.stVal' \
  --fc ST \
  --value 0 \
  --type bool \
  --verify

iec61850_client --host 192.0.2.10 control inspect \
  --object HV_3_AH_04CTRL/BIGGIO1.SPCSO10

iec61850_client --host 192.0.2.10 control operate \
  --object HV_3_AH_04CTRL/Q0GAPC10.SPCSO \
  --value 1 \
  --type bool \
  --yes

iec61850_client --host 192.0.2.10 control operate \
  --object HV_3_AH_04CTRL/Q0GAPC11.SPCSO \
  --value 1 \
  --type bool \
  --yes
```

Those two objects are `direct-with-normal-security`. Each command writes only that object's `Oper` and updates only its `stVal`. stderr for the close command is:

```text
action=operate HV_3_AH_04CTRL/Q0GAPC10.SPCSO 1
action=release
```

The close command without `--yes` does not operate. It reads `HV_3_AH_04CTRL/Q0GAPC10.SPCSO.stVal` with `FC=ST` and prints that row with the `get object` CSV header. stderr is the notice, then `action=get object`, then `action=release`:

```sh
iec61850_client --host 192.0.2.10 control operate \
  --object HV_3_AH_04CTRL/Q0GAPC10.SPCSO \
  --value 1 \
  --type bool
```

```text
存在控制，但是该控制未生效，需要添加确认标识 --yes
action=get object
action=release
```

Report. `--type` defaults to `RP`. `--entry-id` is 16 hex digits.

```sh
iec61850_client --host 192.0.2.10 subscribe report \
  --ld HV_3_AH_04CTRL \
  --ln LLN0 \
  --report rcb01 \
  --type BR \
  --entry-id 0001020304050607
```

GOOSE and sampled values. `--test=false` and `--simulation=false` each need `--live`.

```sh
iec61850_client goose list --scl station.scd

iec61850_client goose publish \
  --iface eth0 \
  --scl station.scd \
  --gocb gcb01 \
  --test=false \
  --live

iec61850_client sv publish \
  --iface eth0 \
  --scl station.scd \
  --svcb msvcb01 \
  --simulation=false \
  --live
```

Server. `--port 0` lets the system pick a port. `--ready-json` is a switch.

```sh
iec61850_server --format text start \
  --scl station.scd \
  --ied-name HV_3_AH_04 \
  --values leaves.json \
  --host 0.0.0.0 \
  --port 102 \
  --max-connections 5 \
  --ready-json
```

`station.scd` is the SCL file (`--scl`): ICD, CID, or SCD. `leaves.json` is the optional `--values` file. It only overrides leaves that already exist in that SCL. The server still does not load either file.

```json
{
  "Leaves": [
    {"Ref": "HV_3_AH_04CTRL/BIGGIO1.SPCSO10.stVal", "FC": "ST", "Type": "BOOL", "Value": {"Raw": false}}
  ]
}
```

## Global flags

Client:

| Flag | Default | Notes |
|------|---------|--------|
| `--host` | unset | Overrides `IEC61850_HOST`. Required for device commands |
| `--port` | `102` | Overrides `IEC61850_PORT` |
| `--timeout` | `10s` | Confirmed-request timeout. Device commands use this instead of the library default of 30s |
| `--connect-timeout` | `10s` | TCP connect timeout |
| `--ied-name` | unset | Overrides `IEC61850_IED_NAME`. Not part of Initiate |
| `--format` | unset | `text`, `json`, or `jsonl`. Reads reject it |

A duration is a bare number of seconds (`10`), seconds with a suffix (`10s`), or milliseconds (`500ms`). Anything else on a device command is exit 2.

Without `--scl`, the client association uses library defaults: TSEL and SSEL `0001`, PSEL `00000001`, PDU size 8192, outstanding calling and called 10, nesting 10, `proposedVersionNumber` 1. Session user requirements are network byte order; duplex is `00 02`.

Server `start` (parsed now; listening waits for Step 5):

| Flag | Default | Notes |
|------|---------|--------|
| `--scl` | required | ICD, CID, and SCD are expected. SSD with no IED is exit 2 once load exists |
| `--ied-name` | first IED with a logical device | Selects which IED to serve |
| `--values` | unset | JSON that only overrides existing leaf values |
| `--host` | `0.0.0.0` | Listen address. Does not come from the SCL IP |
| `--port` | `102` | `0` lets the system pick a port |
| `--max-connections` | `5` | Max simultaneous associations |
| `--ready-json` | off | After listen succeeds, one ready JSON line on stdout |
| `--format` | unset | `text`, `json`, or `jsonl`. Unknown value is exit 2 |

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success. `--help` is also 0 |
| 1 | Failure that does not fit a class below |
| 2 | Bad usage or input: flags, references, or config; unknown `--format`; a file that will not open; a publish or subscribe control block with no usable address. A `GSE` with no usable retransmission time, or a samples-per-cycle sample mode, is also 2 and no frame is sent. `--test=false` / `--simulation=false` without `--live` is also 2. `control operate` without `--yes` is 2 only when the status read cannot start |
| 3 | Connect failed, or the link dropped after connect. A peer Abort, or a report subscription that loses the link, is 3 |
| 4 | Connect or request timeout. GOOSE/SV capture or subscribe with both `--count` and `--duration` that does not fill the count in time is also 4. Both flags default to 0, which means run until Ctrl+C |
| 5 | Not found: object, file, data set, IED, report control block, journal, GOOSE/SV control block, or interface. Includes object-non-existent, and a logical device missing from this connection's domain list (`type=NOT_ON_DEVICE`) |
| 6 | Rejected or not permitted. Includes an MMS Confirmed-Error or Reject other than object-non-existent, a failed select, operate, or CommandTermination, a rejected GOOSE/SV control-block write, and a raw socket without permission |
| 7 | An extra read-back does not match, and only when a check flag was set: `set object --verify`, `control operate --confirm-ref`, or a control-block set that wrote `--enable` or `--disable` and then read GoEna/SvEna differently. Without those flags there is no extra read-back |
| 8 | The device does not support the service, or this operating system cannot do it. Capture, subscribe, and publish off Linux are 8 |
| 130 | Interrupted by Ctrl+C or SIGTERM |

When several rows fail, the process returns one code: the largest of 8, 6, 5, 4, 3, 2, and 1.

The client returns 0 through 6. `set object --verify` can return 7. Exit 8 and 130 are not produced by the commands that run today. The server shell returns 0, 1, and 2. Once Step 5 listens, bad SCL or `--values` is 2, and SIGINT or SIGTERM is 130.

## Error text

These lines are the stderr text, including the trailing newline the process prints. They are Chinese on purpose.

`control operate` without `--yes`. The control is not sent. A `LD/LN.DO` object is read as `LD/LN.DO.stVal` with `FC=ST`. On a successful read, stderr is:

```text
存在控制，但是该控制未生效，需要添加确认标识 --yes
action=get object
action=release
```

`--test=false` or `--simulation=false` without `--live` (exit 2, no frame):

```text
关闭测试位或仿真位需要添加 --live
```

A device command with neither `--host` nor `IEC61850_HOST` (exit 2):

```text
需要 --host 或环境变量 IEC61850_HOST
```

`set object` with `FC=CO` (exit 2, no connect). stderr is both lines:

```text
action=write
拒绝 FC=CO
```

Unknown `--format` (exit 2):

```text
未知的 --format
```

`get object`, `get ln`, `get ds`, or `get journal` given `--format` when the name is one of `text`, `json`, or `jsonl` (exit 2). Reads stay CSV:

```text
读值固定 CSV，不用 --format
```

`--entry-id` that is not exactly 16 hexadecimal characters (exit 2):

```text
EntryID 须为 16 个十六进制字符
```

Unknown `--type` (exit 2):

```text
未知的 --type
```

`--timeout` or `--connect-timeout` that a device command cannot parse (exit 2):

```text
无法解析超时
```

`--value` does not match `--type` (exit 2, no connect):

```text
无法解析 --value
```

`get object --object` is not `LD/LN.DO.DA` (exit 2). `--ln` uses `引用格式须为 LD/LN`, and `--dataset` uses `引用格式须为 LD/LN.数据集名`:

```text
引用格式须为 LD/LN.DO.DA
引用格式须为 LD/LN
引用格式须为 LD/LN.数据集名
```

`get object` has neither `--object` nor `--csv`, sets both, or sets `--object` without `--fc` (exit 2):

```text
需要 --object 和 --fc，或 --csv
--object 与 --csv 互斥
需要 --fc
```

`--csv` cannot be opened, names a live column, or does not have a `reference` and `fc` column on every row (exit 2):

```text
无法打开 --csv
CSV 不能包含 value、type、quality 或 time
CSV 须带 reference 和 fc 列，且每行列数与表头一致
```

`set object --verify` read-back differs (exit 7):

```text
读回不一致
```

`control operate --yes` on a `status-only` object (exit 6):

```text
status-only，对象不可控
```

A client or server command that parsed and passed the checks above, and is not wired yet (exit 1). That is `subscribe report`, `control inspect`, `control operate --yes` when ctlModel is not `direct-with-normal-security` or `status-only`, `get journal`, GOOSE/SV commands once their guards pass, and server `start --scl`:

```text
尚未实现
```

Server `start` without `--scl` is a clap usage error (exit 2).
