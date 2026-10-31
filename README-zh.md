# iec61850-cmd

纯 Rust 的 IEC 61850 客户端和服务端。库的包名是 `iec61850-cmd`（`crates/iec61850`）。客户端命令是 `iec61850_client`（包名 `iec61850-cmd-client`）。服务端命令是 `iec61850_server`（包名 `iec61850-cmd-server`）。都不调用 libiec61850。

验收是和 libiec61850 开源模拟装置互通，不声称和标准全文一致。协议库和命令行分开：库负责报文，命令行负责参数、输出、退出码和护栏。

## 构建

```sh
cargo build -p iec61850-cmd-client -p iec61850-cmd-server
cargo run -p iec61850-cmd-client --bin iec61850_client -- version
cargo run -p iec61850-cmd-server --bin iec61850_server -- version
```

`iec61850_client version` 打印 `iec61850_client 0.0.1`，退出码 0。`iec61850_server version` 打印 `iec61850_server 0.0.1`，退出码 0。`--help` 也是 0。

## 现在能跑的

客户端 `version` 打印包版本。

下面这些客户端命令会建立一条关联，使用库加载的模型，退出前发 MMS Conclude。stderr 先打一行 `action=<命令>`。`set object --verify` 在读回前再打 `action=read`。两条路径最后都打 `action=release`。连接被拒绝或连上之后断开是退出码 3。建连超时或确认请求超时是退出码 4。这些命令已与 `iec61850ctl` 对照过，对端用的是该项目的 `e2e/testdata/interop.icd`。本仓库没有 libiec61850 服务端程序，所以还没对那个对端跑过。

- `list lds` 和 `list lns` 一行一个名字，按序排列。`list dos` 只打数据对象名。`list das` 打 `FC 名字 类型`。类型是 GetVariableAccessAttributes 的短名，例如 `bool`、`bit_string`、`utc_time`、`visible_string`。`--do` 可以写对象名，也可以写 `FC.DO`。`list dss` 打 `LD/LN.数据集名`。`list reports` 打 `LD/LN.BR.名称` 或 `LD/LN.RP.名称`。`list journals` 在对端返回名字时打 `LD/名称`。对端用 Confirmed-Error 拒绝这份名字列表时是退出码 6，不会当成空列表成功。互通用的那台装置没有记录本服务，回答 VMD-state 4。点名的逻辑设备、节点或数据对象不在模型里，退出码 5，stdout 为空。`find path` 打 `LD/LN.DO.DA FC`，没有对上是退出码 5。装置以 object-non-existent 回答（MMS 数据访问错误 10，或 access 类服务错误 2）同样是退出码 5。其它数据访问错误，以及不是 object-non-existent 的 Confirmed-Error 或 Reject，是退出码 6。
- `get object`、`get ln`、`get ds` 打 CSV，标头是 `reference,fc,value,type,quality,time`。布尔值是 `0` 或 `1`。type 列是 MMS 名，例如 `BOOLEAN`、`INTEGER`、`FLOATING_POINT`、`BIT_STRING`、`UTC_TIME`。品质全 0 是 `good:0000`。没有 `q` 或 `t` 是 `-`。时间是 UTC 的 RFC 3339。`stVal` 和 `Op.general` 的行，包括数据集成员，会再读同一数据对象上的 `q` 和 `t`，填进 `quality` 和 `time`。逻辑设备不在这次关联的 domain 列表里时，`type=NOT_ON_DEVICE`，退出码 5。节点或数据集不在时，`type=NOT_FOUND`，退出码 5。`get ds` 的 Read 是 `specificationWithResult=true`（8-1 Table 45）。响应里的访问说明是数据集名；成员顺序仍来自数据集目录。
- 装载模型时，缓冲报告控制块可以是第 1 版的 13 个成员（到 `TimeOfEntry` 为止，没有 `ResvTms`），也可以是第 2 版的 14 个成员，或 15 个成员时丢掉末尾的 `Owner`。非缓冲块是 11 个成员，或 12 个成员时丢掉 `Owner`。不认这 13 个成员时，只有这种控制块的装置会让关联失败，包括 `list lds`。会话用户需求是 ISO 8327 的 16 位值，按网络字节序。本客户端请求的双工功能单元在线上是 `00 02`。
- `set object` 写标量。stderr 只有一行 `action=write`。`--verify` 再打 `action=read` 并读回。对不上是退出码 7，stderr 含 `读回不一致`。

`control operate` 没带 `--yes` 时，先打印 `存在控制，但是该控制未生效，需要添加确认标识 --yes`，再读当前状态。对象写成 `LD/LN.DO` 时，读的是 `LD/LN.DO.stVal`，`FC=ST`，CSV 与 `get object` 相同。退出码跟这次读取走。缺主机、引用不合法或写了 `--format` 仍是退出码 2，提示照样打出。带 `--yes` 的 `control operate` 会读 `ctlModel`。整数 1（`direct-with-normal-security`）写 `Oper`（`ctlVal`、origin 为 `remote-control`、`ctlNum`、`T`、`Test=false`、`Check` 全 0）并退出 0。stderr 是 `action=operate`，后面带对象和值。合闸是 `HV_3_AH_04CTRL/Q0GAPC10.SPCSO`，分闸是 `HV_3_AH_04CTRL/Q0GAPC11.SPCSO`；每次执行只改该对象自己的 `stVal`。引用以 `.Oper.ctlVal` 结尾时是同一个对象。`status-only` 是退出码 6，打印 `status-only，对象不可控`。其它 ctlModel、`control inspect`、`subscribe report`、`get journal` 仍打印 `尚未实现`，退出码 1。库里没有报告订阅循环、没有 Select 或 SelectWithValue 状态机，也没有 ReadJournal。GOOSE 和 SV 的列出、抓包、订阅、发布同样如此：还没有 SCL 读取，也没有原始以太网套接字。`--yes` 只给 `control operate`。其余命令不接受这个旗标。发布仍先检查 `--test` / `--simulation` 和 `--live`，旗标不对是退出码 2，不会发帧。

`goose sniff`、`goose subscribe`、`goose gocb`、`sv list`、`sv sniff`、`sv subscribe`、`sv svcb` 还没有进客户端二进制。clap 把这些名字当成用法错误（退出码 2）。

服务端 `version` 打印包版本。服务端 `start` 必须带 `--scl`。带了 `--scl` 后会解析 PLAN_server.md 里的旗标，然后打印 `尚未实现`，退出码 1。现在还不读 SCL，也不监听。PLAN_server.md 的 Step 5 要等客户端 Step 3 和 Step 4 勾完再开始。

## 命令

```text
iec61850_client
├── list lds
├── list lns|dos|das|dss|reports|journals
├── tree
├── find path --ln <节点名> --path <数据名>
├── get object|ln|ds|journal
├── set object --object <引用> --fc <fc> --value <值> --type <类型> [--verify]
├── subscribe report --ld <逻辑设备> --ln <节点> --report <名称> [--type RP|BR] [--entry-id <16 位十六进制>]
├── control inspect --object <引用>
├── control operate --object <引用> --value <值> --type <类型> [--yes]
├── goose list --scl <文件>
├── goose publish --iface <网卡> --scl <文件> --gocb <名称> [--test] [--live]
├── sv publish --iface <网卡> --scl <文件> --svcb <名称> [--simulation] [--live]
└── version

iec61850_server
├── start --scl <文件> [--ied-name] [--values] [--host] [--port] [--max-connections] [--ready-json]
└── version
```

要连装置的命令需要 `--host` 或 `IEC61850_HOST`。`goose list` 以及 `goose` / `sv` 的 `publish` 不需要。`--ied-name` 会解析，不放进 MMS Initiate。`list lds` 还不会按它过滤。

`set object` 和 `control operate` 的 `--type` 用 `bool`、`int`、`uint`、`float`、`enum`、`string`，大小写不敏感。`boolean`、`integer`、`unsigned`、`double` 分别当成 `bool`、`int`、`uint`、`float`。

`--yes` 只属于 `control operate`。没带就不下发控制：stderr 打错误原文里的那句提示，并改为读取状态值。带了 `--yes` 时，`direct-with-normal-security` 会下发 Operate。列出、读取、写值、报告和 GOOSE/SV 不接受 `--yes`。列出和读取只取参数。`--test` 和 `--simulation` 默认真。`--test=false` 或 `--simulation=false` 还必须带 `--live`。只写 `--live` 不会把位关掉。发布通过这些检查之后仍然不会发帧。

## 案例

`192.0.2.10` 和 `HV_3_AH_04*` 只说明命令怎么写。客户端的全局旗标可以写在子命令前面，加到任意一条上。下面每条都把该二进制接受的旗标写全。

客户端全局旗标。`--timeout 10` 与 `10s` 相同。`--format` 取 `text`、`json` 或 `jsonl`。

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

列出、整棵树和查找：

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

读值。`--object` 配 `--fc`，或者 `--csv`，不能同时写。

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

`points.csv` 是 `--csv` 的输入。必填列是 `reference` 和 `fc`。其它列按原顺序原样带回。不要写 `value`、`type`、`quality`、`time`；这四列由客户端补在最后。

```csv
name,reference,fc
SPCSO10,HV_3_AH_04CTRL/BIGGIO1.SPCSO10.stVal,ST
general,HV_3_AH_04PROT/OvlPTOC1.Op.general,ST
```

`get object --csv points.csv` 打出上面这张表，实时列在最后。单点 `get object`、`get ln`、`get ds` 用同样的实时列，没有额外的输入列。布尔值是 `0` 或 `1`。品质全 0 是 `good:0000`。时间是 UTC 的 RFC 3339。没有 `q` 或 `t` 是 `-`。

```csv
name,reference,fc,value,type,quality,time
SPCSO10,HV_3_AH_04CTRL/BIGGIO1.SPCSO10.stVal,ST,0,BOOLEAN,good:0000,2026-10-29T02:23:01.123Z
general,HV_3_AH_04PROT/OvlPTOC1.Op.general,ST,0,BOOLEAN,good:0000,2026-10-29T02:23:01.123Z
```

```csv
reference,fc,value,type,quality,time
HV_3_AH_04CTRL/BIGGIO1.SPCSO10.stVal,ST,0,BOOLEAN,good:0000,2026-10-29T02:23:01.123Z
```

`get journal` 还没接上。接上之后 stdout 是这份 CSV，不用 `--format`：

```csv
entry_id,time,tag,value
0001020304050607,2026-10-29T02:23:01.123Z,HV_3_AH_04CTRL/BIGGIO1$ST$SPCSO10$stVal,0
```

写值和控制。这里的 `--type` 是 `bool`。`--verify` 会把该点读回来。

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

这两个对象是 `direct-with-normal-security`。每条命令只写该对象的 `Oper`，也只更新它自己的 `stVal`。合闸那条的 stderr 是：

```text
action=operate HV_3_AH_04CTRL/Q0GAPC10.SPCSO 1
action=release
```

合闸那条不写 `--yes` 时不下发控制。它读取 `HV_3_AH_04CTRL/Q0GAPC10.SPCSO.stVal`，`FC=ST`，stdout 用 `get object` 的 CSV 标头打出这一行。stderr 先是提示，然后是 `action=get object`，最后是 `action=release`：

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

报告。`--type` 默认 `RP`。`--entry-id` 是 16 个十六进制字符。

```sh
iec61850_client --host 192.0.2.10 subscribe report \
  --ld HV_3_AH_04CTRL \
  --ln LLN0 \
  --report rcb01 \
  --type BR \
  --entry-id 0001020304050607
```

GOOSE 和采样值。`--test=false` 和 `--simulation=false` 都要再带 `--live`。

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

服务端。`--port 0` 让系统选端口。`--ready-json` 是开关。

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

`station.scd` 是 `--scl` 用的 SCL 文件，可以是 ICD、CID 或 SCD。`leaves.json` 是可选的 `--values`。它只覆盖这份 SCL 里已经有的叶子。服务端现在仍不加载这两个文件。

```json
{
  "Leaves": [
    {"Ref": "HV_3_AH_04CTRL/BIGGIO1.SPCSO10.stVal", "FC": "ST", "Type": "BOOL", "Value": {"Raw": false}}
  ]
}
```

## 全局旗标

客户端：

| 旗标 | 默认 | 说明 |
|------|------|------|
| `--host` | 不写 | 盖过 `IEC61850_HOST`。要连装置的命令必填 |
| `--port` | `102` | 盖过 `IEC61850_PORT` |
| `--timeout` | `10s` | 确认请求超时。要连装置的命令用这个值，不用库里 30 秒的默认请求超时 |
| `--connect-timeout` | `10s` | TCP 建连超时 |
| `--ied-name` | 不写 | 盖过 `IEC61850_IED_NAME`。不放进 Initiate |
| `--format` | 不写 | `text`、`json` 或 `jsonl`。读值拒绝这个旗标 |

时间写成裸秒数（`10`）、带后缀的秒（`10s`）或毫秒（`500ms`）。要连装置的命令解析不了就是退出码 2。

不带 `--scl` 时，客户端关联用库的默认值：TSEL 和 SSEL 为 `0001`，PSEL 为 `00000001`，PDU `8192`，outstanding Calling/Called 均为 `10`，嵌套 `10`，`proposedVersionNumber` 为 `1`。会话用户需求按网络字节序；双工是 `00 02`。

服务端 `start`（现在只解析；监听等 Step 5）：

| 旗标 | 默认 | 说明 |
|------|------|------|
| `--scl` | 必填 | 预期 ICD、CID、SCD。没有 IED 的 SSD 在真正加载后是退出码 2 |
| `--ied-name` | 第一台带逻辑设备的 IED | 选定要服务的 IED |
| `--values` | 不写 | 只覆盖已有叶子初值的 JSON |
| `--host` | `0.0.0.0` | 监听地址。不用工程文件里的 IP 替换本机绑定 |
| `--port` | `102` | `0` 表示系统选端口 |
| `--max-connections` | `5` | 同时关联上限 |
| `--ready-json` | 关 | 监听成功后向 stdout 打一行就绪 JSON |
| `--format` | 不写 | `text`、`json` 或 `jsonl`。未知值是退出码 2 |

## 退出码

| 退出码 | 含义 |
|--------|------|
| 0 | 成功。`--help` 也是 0 |
| 1 | 失败，但分不到下面任何一类 |
| 2 | 用法或输入不对：旗标、引用、配置不合法，未知的 `--format`，工程文件打不开，或发布、订阅用的控制块没有可用地址。`GSE` 没有可用的重传时间，或采样模式是每周波点数，也是 2，帧不发出，stderr 用错误原文里已写死的句子。`--test=false` / `--simulation=false` 时没带 `--live`，同样是 2。`control operate` 没带 `--yes` 时，只有状态读启动不了才是 2 |
| 3 | 连接失败，或连上之后断开。对端 Abort、报告订阅中途拆链，都是 3 |
| 4 | 连接或请求超时。GOOSE/SV 抓包、订阅写了 `--count` 和 `--duration`，时限内没凑满条数，也是 4。两个旗标默认都是 0，不写则一直到 Ctrl+C |
| 5 | 找不到：对象、文件、数据集、IED、报告控制块、记录本、GOOSE/SV 控制块或网卡。含装置回答的 object-non-existent，以及逻辑设备不在本次 domain 列表（`type=NOT_ON_DEVICE`） |
| 6 | 被拒绝或没有权限。含不是 object-non-existent 的 MMS Confirmed-Error 或 Reject，选择、执行或 CommandTermination 失败，GOOSE/SV 控制块写被拒，以及网卡原始套接字没有权限 |
| 7 | 额外读回对不上，且只在写了核对旗标时发生：`set object --verify`、`control operate --confirm-ref`，以及控制块 set 写了 `--enable` 或 `--disable` 后 GoEna/SvEna 读回与请求不一致。不写这些旗标就不做额外读回 |
| 8 | 装置不支持该服务，或当前系统做不了这件事。非 Linux 上的网卡抓包、订阅、发布是 8 |
| 130 | 被 Ctrl+C 或 SIGTERM 打断 |

某一行读失败时，value 为空、type 写原因，stdout 仍打出这一行。多行失败时进程只回一个退出码，在 8、6、5、4、3、2、1 里取数字最大的那一类，不另造号码。

客户端实际会返回 0 到 6。`set object --verify` 读回不一致时可以返回 7。现在能跑的命令还不返回 8 和 130。服务端空壳会返回 0、1、2。服务端真正监听后，工程文件或 `--values` 不合法是 2，收到 SIGINT 或 SIGTERM 是 130。

## 错误原文

下面这些行就是 stderr 的原文，进程会带上末尾换行。

`control operate` 没带 `--yes`。控制不下发。对象是 `LD/LN.DO` 时读 `LD/LN.DO.stVal`，`FC=ST`。读成功时 stderr 是：

```text
存在控制，但是该控制未生效，需要添加确认标识 --yes
action=get object
action=release
```

`--test=false` 或 `--simulation=false` 却没带 `--live`（退出码 2，不发帧）：

```text
关闭测试位或仿真位需要添加 --live
```

要连装置的命令既没有 `--host` 也没有 `IEC61850_HOST`（退出码 2）：

```text
需要 --host 或环境变量 IEC61850_HOST
```

`set object` 的 `FC=CO`（退出码 2，不连网）。stderr 是这两行：

```text
action=write
拒绝 FC=CO
```

未知的 `--format`（退出码 2）：

```text
未知的 --format
```

`get object`、`get ln`、`get ds`、`get journal` 写了 `--format`，且值是 `text`、`json`、`jsonl` 之一（退出码 2）。读值固定 CSV：

```text
读值固定 CSV，不用 --format
```

`--entry-id` 不是恰好 16 个十六进制字符（退出码 2）：

```text
EntryID 须为 16 个十六进制字符
```

未知的 `--type`（退出码 2）：

```text
未知的 --type
```

要连装置的命令解析不了 `--timeout` 或 `--connect-timeout`（退出码 2）：

```text
无法解析超时
```

`--value` 和 `--type` 对不上（退出码 2，不连网）：

```text
无法解析 --value
```

`get object --object` 不是 `LD/LN.DO.DA`（退出码 2）。`--ln` 用 `引用格式须为 LD/LN`，`--dataset` 用 `引用格式须为 LD/LN.数据集名`：

```text
引用格式须为 LD/LN.DO.DA
引用格式须为 LD/LN
引用格式须为 LD/LN.数据集名
```

`get object` 既没有 `--object` 也没有 `--csv`、两个都写了，或写了 `--object` 却没有 `--fc`（退出码 2）：

```text
需要 --object 和 --fc，或 --csv
--object 与 --csv 互斥
需要 --fc
```

`--csv` 打不开、表头里有实时列，或没有 `reference` 和 `fc` 列、某行列数和表头不一致（退出码 2）：

```text
无法打开 --csv
CSV 不能包含 value、type、quality 或 time
CSV 须带 reference 和 fc 列，且每行列数与表头一致
```

`set object --verify` 读回对不上（退出码 7）：

```text
读回不一致
```

`control operate --yes` 打到 `status-only` 对象（退出码 6）：

```text
status-only，对象不可控
```

客户端或服务端命令已经解析、上面的检查也过了，但还没接上（退出码 1）。这是 `subscribe report`、`control inspect`、`control operate --yes` 的 ctlModel 不是 `direct-with-normal-security` 或 `status-only`、`get journal`、护栏通过之后的 GOOSE/SV，以及服务端 `start --scl`：

```text
尚未实现
```

服务端 `start` 没带 `--scl` 是 clap 的用法错误（退出码 2）。
