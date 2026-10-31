# iec61850-cmd 客户端

服务端见 [PLAN_server.md](PLAN_server.md)。等本文件 Step 3 和 Step 4 勾完再开始服务端。

## Description

做一个纯 Rust 的 IEC 61850 客户端命令行 `iec61850_client`。库负责协议，命令行负责参数、输出、退出码和护栏。不调用 libiec61850，也不用 CGO。代码从 `iec61850_1` 演进为 `crates/iec61850`。`iec61850ctl` 和 `iec61850/` 只读，不参与编译。`crates/iec61850_server` 在本文件里只留空壳。

验收是和 libiec61850 开源模拟装置互通，不声称和标准全文或一致性实验室一致。案例里的 `HV_3_AH_04*` 和 `192.0.2.10` 只说明命令怎么写。

客户端要做到：连上、浏览、按 CSV 读值（含品质和时间）、标量写、订阅普通报告和缓冲报告、查看并执行控制、按命令查看装置记录、正常断开。读值只在 stdout。程序正在做什么默认打在 stderr，每次 MMS 调用前一行 `action=`，不另设日志开关。GOOSE 和采样值排在这些 MMS 命令之后，网卡先只做 Linux。

不做：HTTP 网关、Caldera 攻防、IEC 61850-10 一致性认证、IEC 62351、IEC 61850-8-2（XMPP）、ACSE 认证、物理量仿真。文件的列表、下载、上传、删除不进第一版命令行，留在库的目标里，不挡住服务端。

## Steps

### Step 1: 工作区与客户端空壳

- [ ] 建立 Cargo workspace，迁入并演进 `iec61850_1` 为 `crates/iec61850`
- [ ] 增加 `crates/iec61850_client`，`iec61850_client version` 能跑
- [ ] `crates/iec61850_server` 先只留空壳，本步不实现服务端
- [ ] 补上 `README.md` 和 `README-zh.md`，两边章节、命令、默认值和错误原文一致

### Step 2: 客户端先用到的公共块

- [ ] 数值、引用、错误：不连网，用抓到的帧做往返
    - [ ] 布尔、品质、UTC 时间、带 FC 的引用能解回去
    - [ ] ServiceError 按 Notes 的退出码表对应，不另造号码
- [ ] 传输和关联：对 libiec61850 完成 Associate 和 Release（S2、S4）。Associate 在 MMS 里叫 Initiate，Release 在 MMS 里叫 Conclude。Initiate 请求提出 PDU `8192`、outstanding Calling/Called 均为 `10`、嵌套 `10`；`proposedVersionNumber` 为 Integer16 值 `1`（ISO 9506 / 现场与 PICS；与谈妥后的 `negotiatedVersionNumber` 同为 1）。须带 `proposedParameterCBB` 与 `servicesSupportedCalling`。之后一律按 Initiate 响应里谈妥的 PDU / outstanding / 嵌套 / CBB 收发，不得继续按请求值硬发。对端谈妥版本不是 1 则关联失败，退出码 3。Abort（S3）两边同名：对端 Abort、本端发 Abort，或 TCP 被拆后，命令行退出码 3。正常结束走 Conclude，不拿 Abort 代替
- [ ] 确认分发：第一块样板是 GetDataValues（S8）。这一步没过之前，不写别的处理函数
- [ ] 在同一条关联上接收 Report。Report 在 MMS 里叫 InformationReport，不走确认分发

### Step 3: 客户端 MMS 命令

- [ ] `list` / `tree`：目录与 libiec61850 一致。`tree` 不带 `--ld` 时列出整台装置。列下一层要带上上一层：`list lns` 要 `--ld`；`list dos`、`list dss`、`list reports` 要 `--ld` 和 `--ln`；`list das` 还要 `--do`。LD/LN/数据集/报告/记录本目录走 MMS `GetNameList`（见 Notes）；`list das` / GetDataDirectory（及同映射的 GetDataDefinition）走 `GetVariableAccessAttributes`，不是 GetNameList。`GetNameList` 在 `moreFollows=true` 时用上一次最后一个标识作 `continueAfter` 再要，直到 `false`
- [ ] `find path --ln <节点名> --path <数据名>`：先把各逻辑设备的目录问下来，再在本地过滤。`--ln` 写一部分也能对上，`--path` 要写全。这不是官方的一条服务。第一版不做 `find bulk`
- [ ] `get object`：单点和 `--csv` 都输出带标头的 CSV，实时列在最后
    - [ ] 逻辑设备不在本次 domain 列表里时，`type=NOT_ON_DEVICE`，退出码 5。`get ln` 和 `get ds` 的逻辑设备同样按这次连接的 domain 列表检查。多行都失败时，stdout 仍打出全部行，进程退出码取这些行里最严重的一类：8、6、5、4、3、2、1，数字越大越优先；同类只出现一次不影响结果
- [ ] `get ln`：只强制 `--ln`（`LD/LN`）。`--fc` 选填，不写则读该节点下全部功能约束。stdout 用和 `get object` 相同的 CSV，一行一个数据
- [ ] `list dss` 用 `GetNameList`（objectClass=NamedVariableList）列出该逻辑设备下的数据集名，线上名字是 `LN$数据集名`，打印成 `LD/LN.数据集名`。`get ds` 只强制 `--dataset`（`LD/LN.数据集名`）：先用 GetDataSetDirectory（MMS GetNamedVariableListAttributes）拿到成员引用顺序，再 GetDataSetValues（MMS Read，`specificationWithResult=TRUE`，8-1 Table 45）读当前值；stdout 用和 `get object` 相同的 CSV，一行一个成员
- [ ] `set object`：要带 `--object`、`--fc`、`--value`、`--type`。`--type` 用输入短名 `bool|int|uint|float|enum|string`，大小写不敏感；`boolean` 当成 `bool`，`integer` 当成 `int`，`unsigned` 当成 `uint`，`double` 当成 `float`。其它词退出码 2。拒绝 `FC=CO`，stderr 有 `action=write`。`--verify` 默认关；写了才在写完后读回同一点，对不上则退出码 7
- [ ] `subscribe report`：要带 `--ld`、`--ln` 和 `--report`。`--type` 默认 `RP`，不写就是这个值。`--report` 写装置列出来的名字。不改写 `DatSet`、`OptFlds`、`TrgOps`、`BufTm`、`IntgPd`、`RptID`，沿用装置已有配置。URCB：先写 `Resv=true`，再写 `RptEna=true`；若 `Resv` 已被其它客户端占用，装置拒绝，退出码 6。BRCB：若写了 `--purge-buf` / `--entry-id` / `--resv-tms`，必须在使能前写；`--entry-id` 是 ACSI EntryID，固定 8 个八位组（恰好 16 个十六进制字符，8-1 §8.1.3.5），否则退出码 2；全 0（`0000000000000000`）是保留值。`--resv-tms` 写 ACSI `ResvTms`（INT16，单位秒）：只允许写 ≥ 0 的值；写负值退出码 2（装置侧可读到 `-1` 表示配置独占，客户端不得写入负值，tissue 300）。使能后只允许再写 `RptEna` 和 `GI`。写在 `RP` 上是用法错误，退出码 2。使能后先读 `TrgOps`，`gi` 为真才写 `GI=true`；为假时不写 GI，stderr 说明，订阅继续。退出时先写 `RptEna=false`；URCB 再写 `Resv=false`。收到的报告是 MMS InformationReport。URCB 的 `SqNum` 按无符号 8 位解，BRCB 按无符号 16 位解；BRCB 使能不把 `SqNum` 置 0（tissue 301）
- [ ] `control inspect`：要带 `--object`。只查看 ctlModel 和控制对象，不需要 `--yes`
- [ ] `control operate`：要带 `--object`、`--value`、`--type`。`--type` 与 `set object` 同一套输入短名。没有 `--yes` 时不下发，退出码 2，并打印 Notes 里的那句提示。`--mode` 与 ctlModel 的对应见 Notes。`--confirm-ref` 默认不写，写了才读该点，形式是 `LD/LN.DO.DA[FC]`，对不上则退出码 7。选择、执行或 CommandTermination 失败是退出码 6
- [ ] Select 或 SelectWithValue 成功、随后 Operate 失败时，在同一次连接里发 Cancel，stderr 写 `action=cancel`。直接执行失败，或选择被拒绝时，不发。不另加 `control cancel`
- [ ] `list journals` 用 GetLogicalNodeDirectory 列出 LOG。不带 `--ld` 时先列出全部逻辑设备，再逐个列出记录本名字。每一行都带上逻辑设备名和记录本名称。带了 `--ld` 只列这一台。`get journal --ld <逻辑设备> <记录本名称>` 才读条目，stdout 固定 CSV，列是 `entry_id,time,tag,value`。不写 `--entry` 时走 QueryLogByTime：`--from` 默认当天 00:00，`--to` 不写则读到最新。写了 `--entry` 时走 QueryLogAfter：EntryID 取自 `--entry`，TimeOfEntry 取自 `--from`；此时 `--from` 不写则发全 0 的 Binary Time，不再套用当天 00:00。其它命令不碰装置记录，也不打开 LogEna
- [ ] 对 libiec61850 模拟装置跑通上述命令。本步不打自己的服务端

### Step 4: 客户端 GOOSE 与采样值

- [ ] 用录好的 GOOSE、SV 帧做编解码往返。编解码不绑操作系统
- [ ] `goose list` 和 `sv list` 只读 `--scl`，不连网、不抓包。每一行带上 IED 名、控制块、APPID、目的地址、VLAN 和数据集。`--ied-name` 不写就列出全部 IED，写了只列这一台。不认识的节点记到 stderr 并跳过。不单做 `scl parse`
- [ ] `goose sniff` 和 `sv sniff` 只强制网卡名，不读工程文件，也不要 `--host`。`--count` 默认 0，表示不限条数。`--duration` 默认 0，表示直到 Ctrl+C。只写了 `--count` 时，凑满就退出码 0。只写了 `--duration` 时，到时就退出码 0。两个都写了且时限内没凑满，退出码 4
- [ ] `goose subscribe` 要 `--iface`、`--scl`、`--gocb`。`sv subscribe` 要 `--iface`、`--scl`、`--svcb`。按工程文件里的地址、APPID，以及 gocbRef 或 svID 过滤。帧里缺 `goID` 时按 tissue 1739 / 8-1 §18.1.1 当作默认 GoID（= 该帧 `gocbRef`，即 GoCB 未配置 GoID 时的缺省）再与 SCL 的 `appID` 比。datSet、goID 或 svID、confRev 对不上，或 `ndsCom` 为真的帧，记到 stderr，不计入 `--count`，命令继续。以太网头或 PDU 里仿真位为真时，没有 `--accept-simulation` 就丢掉，不计入 `--count`。停止规则与抓包相同
- [ ] 发布要网卡名、`--scl`、控制块名和 `--yes`，不要 `--host`。先选定 IED，再在它的 `ConnectedAP` 下用逻辑设备的 `inst` 和控制块名对上 `GSE` 或 `SMV`。目的地址、十六进制 APPID、VLAN、MinTime、MaxTime 取自那里，不取自 `GSEControl` 的 `appID`。GOOSE 的以太网类型是 `0x88B8`，采样值是 `0x88BA`。采样率和重传规则见 Notes。源 MAC 用这块网卡自己的地址。没写的成员用该类型的零值，SV 品质为 good。`--value` 选填，可重复，写成 `序号=值` 或 `LD/LN.DO.DA[FC]=值`。不做正弦或其它物理量仿真。`--duration` 默认 0，直到 Ctrl+C，到时退出码 0
- [ ] GOOSE 的 `--test` 默认真。SV 的 `--simulation` 默认真。写成 `false` 时必须再带 `--live`，否则退出码 2，帧不发出，并打印 Notes 里的那句提示。只写 `--live` 不会把位关掉。没有 `--yes` 时同样退出码 2，帧不发出
- [ ] 同一台机器的两块网卡或一对虚拟网卡上，一边发一边收。默认 GOOSE `--test=true` 时 PDU/Reserved1 仿真位为真，接收侧要加 `--accept-simulation` 才收下；品质属性里的 test 位不挡订阅。SV 同理看帧上的仿真位
- [ ] `goose gocb get|set` 和 `sv svcb get|set` 走 MMS，要 `--host`、`--ld` 和控制块名，不读工程文件。set 只写 `--enable` / `--disable`、`--go-id` 或 `--msv-id`、`--dat-set`。`GoEna` / `SvEna` 随时可写。`GoID` / `MsvID` 和 `DatSet`：按 7-2 SetGoCBValues / SetMSVCBValues，仅在未使能时可写，使能最后写（使能为真时装置须拒绝）。8-1 Table 73 把 GoCB 的 GoID/DatSet 列成 `r`，与 7-2 / tissue 1582 的可写语义不一致；本项目按 7-2 可写实现（与 libiec61850 互通），装置若只读拒绝则退出码 6。不写 `ConfRev`、`NdsCom`、`DstAddress`、`MinTime`、`MaxTime`、`FixedOffs`（只读）。装置侧对 `DatSet` 的成功写入（含写成相同值）仍会使 `ConfRev` 加 1（7-2 / tissue 1582）。GoCB 上 `GoID` 未配置时的缺省是该控制块对象引用（8-1 §18.1.1）；get 若读到空则按此理解。写了 `--enable` 或 `--disable` 后读回不一致，退出码 7。装置拒绝为 6
- [ ] 控制块不在工程文件里，退出码 5。文件打不开，或发布、订阅用的控制块没有可用地址，退出码 2，帧不发出
- [ ] 其它系统上抓包、订阅、发布直接返回不支持，退出码 8

### Notes

行为按 IEC 61850-6 Edition 2.2 和 7-2 / 7-3 / 7-4 / 8-1 / 9-2 Edition 2.1 的常用子集来写。第 6 部分的命名空间名字是 `IEC 61850-6:2007C5`。XML 命名空间仍是 `http://www.iec.ch/61850/2003/SCL`；根元素固定 `version="2007"`、`revision="C"`、`release="5"`。SCL 模式已在 `standards/iec-free-code-components/IEC_61850-6.SCL.2007C5.full/`。7-2、7-3、7-4、8-1、9-2 的 PDF 全文仍不在仓库里，购买页见 `standards/README.md`。核对记录见 `docs/superpowers/specs/2026-10-29-iec61850-official-check.md`。

仓库：

```
crates/iec61850/src/
├── data/
├── path/
├── transport/
├── association/
├── mms/dispatch.rs
├── error.rs
├── model/
└── ethernet/
```

服务处理只调用 path、data、dispatch，不碰 TCP。报告是 ACSI 的 Report，在 MMS 里叫 InformationReport，不走确认分发。S35、S45 走以太网，它们的控制块读写仍走 MMS。S48 单播采样留到服务端计划的 Step 6。

| 项 | 决定 |
|----|------|
| 顺序 | 先做完本文件 Step 3 和 Step 4，再做 [PLAN_server.md](PLAN_server.md) |
| 验收装置 | libiec61850 开源模拟装置 |
| 不认识的段落 | 客户端读工程文件时，stderr 记下节点名并跳过，不拒绝整份文件 |
| 解析深度 | Step 4 从控制块读数据集和成员，从对上的 `GSE` / `SMV` 读地址和 APPID。不单做 `scl parse` |
| 报告 | 客户端 URCB 和 BRCB 一起做 |
| 控制 | 见下方 ctlModel 对照表。SBOw 是对象，不是模型名 |
| 确认标识 | `operate` 和 GOOSE/SV 发布都必须带 `--yes`。`--test` 和 `--simulation` 默认真。写成 `false` 时必须再带 `--live`；只写 `--live` 不会把位关掉 |
| 取消 | 不另加 `control cancel`。Select 或 SelectWithValue 成功、Operate 失败时，由 `operate` 在同一次连接里发出。直接执行失败，或选择被拒绝时，不发 |
| 装置记录 | 不设 `--journal`。浏览、读、写、报告、控制不去问。`list journals` 只列名字。`get journal` 的 CSV 列是 `entry_id,time,tag,value`。都不打开装置上的日志开关 |
| 程序日志 | 默认开启，只走 stderr。不做成 `--log-level` |
| 文件 | 第一版命令行不做列表、下载、上传、删除，也不单做 `scl parse` |
| 通用写 | `set object` 拒绝 `FC=CO`。控制命令才写 CO。写值 `--type` 用输入短名 `bool\|int\|uint\|float\|enum\|string`。读出的 type 列仍用 MMS 名。`--verify` 和 `operate` 的 `--confirm-ref` 默认都不做额外读回 |
| 网卡 | 先只做 Linux。MMS 不限系统。网卡收发在其它系统上退出码 8 |
| 传输 | 明文 TCP。TLS 只留配置口，不验收 |
| 引用 | 命令行用 `LD/LN.DO.DA` 加 `--fc`。库内可用 `LN$FC$DO$DA` |
| 输出 | 退出码用下面的对照表（0–8、130）。读值和读记录本固定 CSV。其余命令默认 text；脚本可用 JSON / JSONL |
| 许可证 | MIT。NOTICE 致谢 `iec61850_1` 原作者 |
| 不做的服务 | S36、S37。S40–S44 GSSE 不实现、不计数。对时用操作系统时钟。定值组、取代、单播采样、定时执行、文件传输见服务端计划 |

官方 ctlModel 五个名字。命令行 `--mode` 是产品短名，对齐 `iec61850ctl`，不是标准条文。`auto` 按装置 ctlModel 选下面某一行。显式 `--mode` 与装置不在同一行时，退出码 2，不下发。`status-only` 在 `auto` 或任意显式模式下都不下发，退出码 6。

| 装置 ctlModel | `--mode auto` 选成 | 允许的显式 `--mode` | 发出的服务 |
|---------------|--------------------|----------------------|------------|
| status-only | （不可执行） | 无 | 不下发 |
| direct-with-normal-security | `direct` | `direct` | Operate |
| direct-with-enhanced-security | `direct` | `direct` | Operate，再等 CommandTermination |
| sbo-with-normal-security | `sbo` | `sbo` | Select，再 Operate |
| sbo-with-enhanced-security | `sbow` | `sbow` | SelectWithValue，再 Operate，再等 CommandTermination |

普通安全的两个模型不等 CommandTermination。增强安全的两个模型必须等，等待受 `--timeout` 约束，超时退出码 4；若选择已经成功，仍按上面的规则发 Cancel。`--confirm-ref` 是额外的状态读回，不是 CommandTermination。

8-1 控制对象映射：Select 是 MMS **读** `{LN}$CO${DO}$SBO`（没有 ctlVal）；SelectWithValue 是 MMS **写** `{LN}$CO${DO}$SBOw`；Operate 写 `{LN}$CO${DO}$Oper`；Cancel 写 `{LN}$CO${DO}$Cancel`。tissue 1288：除 Select 读 `$SBO` 外，不要对其它 `FC=CO` 发 GetDataValues；通用 `get object` / `set object` 仍拒绝 CO。Select 肯定：读回非空 Visible-string；客户端按 tissue 1753 接受任意非空值（含 `CO_CtrlObjectRef`、`$SBO`、`$Oper` 等历史写法）。否定：仍是读成功，但值为空串 / NULL（8-1 §20.5.5），不是 MMS 服务错误。

Operate / SelectWithValue 结构成员顺序（8-1 Table 85/90）：`ctlVal`、可选 `operTm`、`origin`、`ctlNum`、`T`、`Test`、`Check`。Cancel 顺序（Table 88）：`ctlVal`、可选 `operTm`、`origin`、`ctlNum`、`T`、`Test`（**没有** `Check`）；`ctlVal` 在 Cancel 上也是必填，与这次要取消的操作值相同。普通安全的 Select 不带 `ctlVal`。`origin.orCat` 不写则是 `remote-control`（整数 3）。`orIdent` 不写则为空八位组串，最长 64 字节。`ctlNum` 从 1 起到 255，再回到 1，不用 0；同一次选择与随后的 Operate、Cancel 用同一个数。`T` 用当前 UTC。`Test` 为假。`Check` 是宽度为 2 的位串：bit0 同期（synchrocheck）、bit1 联锁（interlock-check），默认都为假。缺了这些字段的控制写，服务端拒绝。本项目第一版不做定时执行，不发 `operTm`。CommandTermination 按 8-1 Table 92 / §20.9：肯定结束是带该控制 `$Oper` 结构的 InformationReport（不是 LastApplError）；否定结束是同一条 InformationReport 里先 VMD 名 `"LastApplError"`（结构顺序 `CntrlObj`、`Error`、`Origin`、`ctlNum`、`AddCause`，Table 96），再跟 `$Oper`。`AddCause`、`ctlNum`、`Error` 记到 stderr，进程退出码仍按失败类别收成 6。

第三方 Rust 库覆盖不到上述效果。本仓库的 crates.io 包名用 `iec61850-cmd`（2026-10-31 核对未被占用）。`iec61850`、`iec61850-rs`、`iec61850-client`、`iec61850-server`、`iec61850-mms` 已被别人发布，不能再用。

`iec61850_1` 已有客户端传输、浏览、数据集、RCB 读写、按路径读写、读文件和 TLS 配置钩子。没有服务端、SCL、控制状态机、装置记录查询、GOOSE/SV、命令行和退出码契约。

```
iec61850_client
├── list lds|lns|dos|das|dss|reports|journals
├── tree
├── find path --ln <节点名> --path <数据名>
├── get object                  # --object + --fc，或 --csv
├── get ln --ln LD/LN [--fc]    # 不写 --fc 则读该节点全部值。CSV 与 get object 相同
├── get ds --dataset LD/LN.名称  # 只此为必填。按成员顺序读值，CSV 与 get object 相同
├── get journal --ld <逻辑设备> <记录本名称> [--from] [--to] [--entry]
├── set object                  # --object、--fc、--value、--type；拒绝 FC=CO。--verify 默认关
├── subscribe report            # --ld、--ln、--report；--type 默认 RP
├── control inspect|operate     # inspect 要 --object；operate 还要 --value、--type、--yes。--confirm-ref 默认不写
├── goose list|sniff|subscribe|publish
├── goose gocb get|set
├── sv list|sniff|subscribe|publish
├── sv svcb get|set
└── version
```

`goose` 和 `sv` 都是 `iec61850_client` 的子命令。列出、抓包、订阅、发布不连装置，不需要 `--host`。`gocb` 和 `svcb` 要连装置。控制块名可以是短名，或 `LLN0.GO.名称`、`LLN0.MS.名称`。

参数分两类。

**必填：** 不写就不知道对哪一个对象操作，或者是已经定过的护栏。要连装置的命令，主机用 `--host` 或环境变量 `IEC61850_HOST`，两边都空则不能连。`goose` / `sv` 的 list、sniff、subscribe、publish 不在此列。另外还有 `find path` 的 `--ln` 和 `--path`、`get object` 的 `--object` 和 `--fc`、`get ln` 的 `--ln`、`get ds` 的 `--dataset`、`set object` 的 `--object`、`--fc`、`--value` 和 `--type`、`subscribe report` 的 `--ld`、`--ln`、`--report`、`control inspect` 的 `--object`、`control operate` 的 `--object`、`--value`、`--type` 和 `--yes`、`get journal` 的 `--ld` 和记录本名称。`goose list` 和 `sv list` 要 `--scl`。抓包要 `--iface`。`goose subscribe` 要 `--iface`、`--scl`、`--gocb`；`sv subscribe` 要 `--iface`、`--scl`、`--svcb`。`goose publish` 要 `--iface`、`--scl`、`--gocb` 和 `--yes`；`sv publish` 要 `--iface`、`--scl`、`--svcb` 和 `--yes`。控制块的 get/set 要 `--ld` 和控制块名，并且要能连上装置。标准没有「省略就用某一台装置、某一块控制块」这种默认值，所以这些不能空着。

**选填：** 不写就用默认值，不能把它们做成必填。要连装置的命令可以再带 `--scl`。带了，就从选定 IED 的 `ConnectedAP/Address` 读取 `OSI-TSEL`、`OSI-SSEL`、`OSI-PSEL` 和 `MMS-Port`，按十六进制字节原样使用，不改长度。`--port` 写了才盖过 `MMS-Port`。文件里没有这些项时，才用端口 `102`、TSEL/SSEL=`0001`、PSEL=`00000001`。不带 `--scl` 时直接用这组值。标准没有默认值、本计划已经定过的，用计划里的值：报告 `--type` 为 `RP`，`get journal` 的 `--from` 为当天 00:00，`--to` 不写则读到最新，`--mode` 为 `auto`，控制的 Test 位为假、Check 不附加检查、时间用当前时间。抓包和订阅的 `--count` 为 0、`--duration` 为 0。发布的 `--duration` 为 0。`--test` 和 `--simulation` 为真。`--entry`、`--ld`（仅 `list journals`）、`--format`、`--dry-run`、`--ied-name`、`--verify`、`--confirm-ref`、发布用的 `--value`、`--live`、`--accept-simulation`、`--enable`、`--disable`、`--go-id`、`--msv-id`、`--dat-set`、`--purge-buf`、`--entry-id`、`--resv-tms` 都是选填；不写表示不用这项，或用上面的默认。

全局旗标：`--host`、`--port`（默认 102）、`--timeout`（默认 10s）、`--connect-timeout`（默认 10s）、`--ied-name`。环境变量 `IEC61850_HOST`、`IEC61850_PORT`、`IEC61850_IED_NAME`，旗标优先。`--ied-name` 不放进 MMS Initiate。连装置时用来对齐逻辑设备名。读工程文件时用来选定 IED：不写，且只有一台 IED 含有要找的控制块，就用那一台；多于一台则退出码 2。写了却找不到该 IED 或该控制块，退出码 5。Initiate 请求里的提议是 PDU `8192`、outstanding Calling/Called `10`、嵌套 `10`、`proposedVersionNumber=1`；实际收发以响应谈妥值为准（版本须为 1）。建连重试默认 0 次。

`list journals` 和 `get journal` 的帮助默认写上：这两条命令自己去看装置记录本，不必再加开关。其它命令不会去看。`list journals` 不带 `--ld` 时列出这次连上的全部逻辑设备里的记录本；带了 `--ld` 只列这一台。每一行都写出逻辑设备名和记录本名称，只列名字，不读条目。`get journal` 必须带 `--ld <逻辑设备>`，记录本名称是后面的位置参数。

不写 `--entry` 时发 QueryLogByTime（MMS ReadJournal 按时间）：`--from` 是开始时间，不写则为本机当天 00:00，再换成 UTC；`--to` 是结束时间，不写则读到最新一条。写了 `--entry` 时发 QueryLogAfter（MMS ReadJournalAfter）：EntryID 是 `--entry` 的十六进制八位组串，须为固定 8 个八位组（恰好 16 个十六进制字符，8-1 §8.1.3.5），否则退出码 2；TimeOfEntry 取 `--from`，不写 `--from` 则发 6 字节全 0 的 Binary Time，不再套用当天 00:00。此时 `--to` 仍作结束边界：装置侧若不支持在 After 查询上截断，客户端在本地丢掉晚于 `--to` 的行。

官方一条记录是 EntryID、TimeOfEntry，再加若干变量。线上的 `TimeOfEntry` 是 MMS BINARY-TIME，固定 6 字节：前 4 字节大端，是当天 0 点起的毫秒；后 2 字节大端，是自 1984-01-01 UTC 起的整天数。全 0 表示该纪元零点。读出后换成 UTC 的 RFC 3339 再打印；`EntryID` 固定 8 个八位组，打印成不带 `0x` 的 16 个十六进制字符。标准没有规定打印格式。stdout 固定 CSV，不用 `--format`，一行一个变量：

```csv
entry_id,time,tag,value
0001020304050607,2026-10-29T02:23:01.123Z,HV_3_AH_04CTRL/BIGGIO1$ST$SPCSO10$stVal,0
```

`entry_id` 是十六进制、不带 `0x`、固定 16 个字符。`time` 是 UTC 的 RFC 3339。`tag` 和 `value` 按装置返回的变量原样写出；一条记录里有几个变量就有几行，条目号和时间重复。没有变量时仍留一行，`tag` 和 `value` 为空。没有记录时只打标头，退出码 0。装置回答 `moreFollows` 时，stdout 不变，stderr 写 `more=1 entry=<最后一条 entry_id> time=<最后一条 time>`，下次用这两个值继续 QueryLogAfter。

读值的 CSV：必填列只有 `reference` 和 `fc`。其它列可选，按输入顺序原样带回。实时列固定在最后，输入里不能用 `value`、`type`、`quality`、`time`。布尔和单比特的 value 用 `0`/`1`。type 列用 MMS 类型名；单点是 `BOOLEAN`，双点是宽度为 2 的 `BIT_STRING`。`set object` 和 `control operate` 的 `--type` 不用 MMS 名，只用输入短名 `bool|int|uint|float|enum|string`。bool 的 `--value` 写 `true`/`false` 或 `0`/`1`。quality 例如 `good:0000`，没有 `q` 时为 `-`。time 是 UTC 的 RFC 3339，没有 `t` 时为 `-`。`stVal` 和 `Op.general` 同时读同一数据对象上的 `q` 和 `t`。`Oper.ctlVal` 只是读 `FC=CO`，时间取 `Oper.T`，品质为 `-`。`--object` 与 `--csv` 互斥。字段按 RFC 4180 加引号。某一行失败时 value 为空、type 写原因，stdout 仍打出这一行。多行失败时进程只回一个退出码：在 8、6、5、4、3、2、1 里取数字最大的那一类。同一行同时有找不到和被拒绝时，也按这个顺序。同装置只看这次连接的 domain 列表。

```sh
iec61850_client get object \
  --host 192.0.2.10 --port 102 \
  --object 'HV_3_AH_04CTRL/BIGGIO1.SPCSO10.stVal' \
  --fc ST
```

```csv
reference,fc,value,type,quality,time
HV_3_AH_04CTRL/BIGGIO1.SPCSO10.stVal,ST,0,BOOLEAN,good:0000,2026-10-29T02:23:01.123Z
```

多行输入可以带可选列，例如 `name`，输出仍把实时列放在最后。案例点：`BIGGIO1` 的 SPCSO10、12、7、6、5、4、16、9、1、17、38、39、41、42、11 的 `stVal`，以及 `OvlPTOC1.Op.general`、`pdGGIO283.SPCSO57.stVal`、`ComGGIO1.SPCSO1.stVal`、`ComGGIO1.SPCSO2.stVal`，FC 都是 `ST`。`Q0GAPC10.SPCSO.Oper.ctlVal` 和 `Q0GAPC11.SPCSO.Oper.ctlVal` 的 FC 是 `CO`。逻辑设备名分别是 `HV_3_AH_04CTRL`、`HV_3_AH_04PROT`、`HV_3_AH_04EXT`、`HV_3_AH_04LD0`。

`control operate` 没带 `--yes` 时，stderr 写下面这一句，退出码 2：

```text
存在控制，但是该控制未生效，需要添加确认标识 --yes
```

带上 `--yes` 之后，按上面的 ctlModel 对照表选择（如果需要）并执行。stderr 的 `action=operate` 带上对象和值。`--dry-run` 默认关。`--confirm-ref` 不写则不做额外的状态读回。GOOSE/SV 发布没带 `--yes` 时同样退出码 2，stderr 写：

```text
发布未生效，需要添加确认标识 --yes
```

GOOSE 写成 `--test=false`、或 SV 写成 `--simulation=false`，但没带 `--live` 时，退出码 2，帧不发出，stderr 写：

```text
关闭测试位或仿真位需要添加 --live
```

报告 `--type` 默认 `RP`。抓包和订阅不写 `--count`、`--duration` 时一直运行，直到 Ctrl+C，退出码 130。

IEC 61850 只规定装置和客户端之间的报文，不规定命令行怎么打印。打印格式是产品约定，对齐 `iec61850ctl`，不是协议条文。

客户端里的 JSON 只给脚本读命令结果。人手看时默认 `text`。一次出完结果的命令用 `json`：`list`、`tree`、`find path`、`set object`、`control`、`goose list`、`sv list`、`goose gocb`、`sv svcb`。一边跑一边出结果的命令用 `jsonl`：`subscribe report`、`goose sniff|subscribe|publish`、`sv sniff|subscribe|publish`。字段名对齐 `iec61850ctl`，不要求逐字节相同。未知的 `--format` 退出码 2，不偷偷改成 text。`get object`、`get ln`、`get ds`、`get journal` 固定 CSV，不用 `--format`。写了 `--format json` 且控制或写值已经发到装置时，即使退出码非 0，stdout 仍打出这一份 JSON。诊断、`action=` 和连接提示只走 stderr。服务端的 `--values` 和 `--ready-json` 见 [PLAN_server.md](PLAN_server.md)。

写文档时不要写错：6 是 Edition 2.2，7-2 / 7-3 / 7-4 / 8-1 / 9-2 的现行合并版是 Edition 2.1。GOOSE 是 8-1 的以太网映射。采样值的 IEC 正文是 9-2，9-2LE 只是 UCA 布局。7-2 用 GetDataValues 这类名字，GetNameList / Read / Write / InformationReport 是 MMS 名。ctlModel 是五个安全模型名，SBOw 是对象。仓库里的 `1MRK511393-UEN` 和 `iec61850-intro.pdf` 都不是 IEC 正文。

GOOSE / SV 发布和订阅读 SCL 时按 2007C5 模式拆开，不要把字段都当成控制块属性：

| 要的内容 | 取自 |
|----------|------|
| 目的 MAC、十六进制 APPID、VLAN-ID、VLAN-PRIORITY | 选定 IED 的 `ConnectedAP` 下，`GSE` 或 `SMV` 的 `Address/P`。`ldInst` 对的是 `LDevice/@inst`，不是 `ldName`；`cbName` 对控制块名 |
| MinTime、MaxTime | 同一条 `GSE`。数字按毫秒；`unit`/`multiplier` 可省略，写了须为 `s`/`m`，不要把数字当成秒 |
| goID | `GSEControl` 的 `appID`。这是字符串，不是上面的十六进制 APPID。Ed2.1：GoID / PDU `goID` 均为 VisString129（最长 129）；旧版曾写 65（tissue 232），本项目按 Ed2.1。超过 129 退出码 2，不截断。SV 的 `svID`（=`smvID`）同为 VisString129 |
| svID、smpRate、smpMod、nofASDU | `SampledValueControl` |
| datSet、confRev、控制块名 | `GSEControl` 或 `SampledValueControl` |

可用地址必须同时有 `MAC-Address` 和 `APPID`。APPID 按十六进制整数读，可以不带 `0x`。官方示例里 SV 的 `4000` 是 `0x4000`。GOOSE 的 APPID 按 8-1 Annex C：Type 1 为 `0x0001`–`0x3FFF`，Type 1A（Trip）为 `0x8000`–`0xBFFF`；`0x0000` 在标准里表示未配置，不算可用地址。SV 为 `0x4000`–`0x7FFF`。目的 MAC 必须是组播地址。GOOSE 还要落在 `01-0C-CD-01-00-00`–`01-0C-CD-01-01-FF`，采样值要落在 `01-0C-CD-04-00-00`–`01-0C-CD-04-01-FF`；这是 8-1 / 9-2 地址表里的推荐范围，也是现场装置常用过滤范围。超出上述任一条件则没有可用地址，退出码 2。8-1 Annex C Table C.1：GOOSE 和采样值在没写 `VLAN-ID` / `VLAN-PRIORITY` 时仍打 802.1Q（TPID=`0x8100`），VID 为 0、优先级为 4（CFI 为 0）。写了 `VLAN-ID` 就用该三位十六进制（含 `000`）；写了 `VLAN-PRIORITY` 就用该值，只能是 `0`–`7`，对不上则退出码 2。订阅端有标签和无标签都收，不拿 VID 或优先级做过滤。MinTime、MaxTime 只使用这条 `GSE` 自己写的数字：类型是 `tDurationInMilliSec`，元素内容按毫秒解。`unit` / `multiplier` 可省略；写了则必须是 `s` / `m`（模式 fixed），否则退出码 2。两条里缺了任意一条，或 MinTime 大于 MaxTime，发布退出码 2，帧不发出，stderr 写 `GSE 未给出可用的重传时间，发布未发出`。不把示例里另一条控制块的 4 ms / 1000 ms 填进来。`type` 为 GOOSE（含缺省）或采样值控制块没写 `confRev` 时，发布退出码 2，不把修订号填成 0。第 6 部分正文要求这两处必填，模式为兼容第一版才标成可选（tissue 1791）。写了则用该无符号整数，包括 0。

`smpMod` 不写则是 `SmpPerPeriod`。这份模式没有额定频率，不能把每周波点数换成时间。`smpMod` 为 `SmpPerPeriod`（含没写）时，发布退出码 2，帧不发出，stderr 写 `采样模式是每周波点数，工程文件没有额定频率，发布未发出`。`SmpPerSec` 时采样间隔是 `1/smpRate` 秒。`SecPerSmp` 时采样间隔就是 `smpRate` 秒。模式里 `smpRate` 与 `nofASDU` 都是必填；缺了任一项，发布退出码 2，不发帧。一帧的时间间隔等于采样间隔乘以 `nofASDU`。订阅不靠这个换算，仍可接收。

`SampledValueControl` 在模式里必须带 `SmvOpts` 元素；缺了发布退出码 2。SV 帧的可选字段按该元素的属性：属性没写时 `sampleSynchronized` 为真，其余为假。字段出现只说明帧里带这个量，不说明已经对时。本项目不做 9-3 / PTP；`smpSynch` 固定发 0（9-2：未同步）。`smpCnt` 从 0 起，每取一个新采样加 1。`SmpPerPeriod` 时每个周波回到 0。`SmpPerSec` 和 `SecPerSmp` 时在 UTC 的秒发生变化时回到 0。SV 的 `--simulation` 只写以太网头 `Reserved1` 最高位（掩码 `0x8000`；SV PDU 没有与 GOOSE 对应的 tag [7]）；默认 `true`。APDU Length 算法与 GOOSE 相同。

`GSEControl` 的 `type` 缺省是 `GOOSE`。`fixedOffs` 缺省为假：为假时按普通 BER 编码，长度字段用最短形式（tissue 1345）；为真时按 8-1 Annex A 定长编码发布。定长里的 `FLOAT32` 仍是 ASN.1 长度 5（首字节指数宽度 8 + 4 字节 IEEE 754），不是长度 4（tissue 817）。订阅一律按带长度的 BER 解，定长帧也能解。`type="GSSE"`、控制块带了 `Protocol`（`R-GOOSE` 或 `R-SV`）、或 SV 的 `multicast="false"` 时，不发以太网帧，退出码 8。根上的 `version` / `revision` / `release` 不是 `2007` / `C` / `5` 时记到 stderr，不因此拒绝整份文件。抓包和订阅也按以太网类型过滤：GOOSE 只收 `0x88B8`，采样值只收 `0x88BA`。

逻辑设备名：`LDevice` 写了 `ldName` 就用它，否则把 IED 名和 `inst` 直接拼在一起，中间不加分隔符。通信段里的 `GSE` / `SMV` 挂在 `ConnectedAP` 上，`ConnectedAP/@iedName` 必须是这台 IED。在这台 IED 里仍对上多于一条时，退出码 2，不发帧。MAC：2007C5 模式要求大写与 `-` 分隔（`tP_MAC-Address`）；本实现也接受 `:` 与小写，发出去仍是这六个字节。APPID 模式为四位大写十六进制；读入时大小写不敏感。控制块没有 `datSet`，或这个名字对不上同一 `LLN0` 里的 `DataSet/@name` 时，发布退出码 2；订阅不按 datSet 过滤。

帧里用来核对的引用按 8-1 的可见字符串来拼：`gocbRef` 是 `{逻辑设备名}/LLN0$GO$` 后接控制块名；datSet 引用是 `{逻辑设备名}/LLN0$` 后接数据集名（`LLN0` 与数据集名之间必须有 `$`，与 iec61850ctl / 现场帧一致，例如 `LD0/LLN0$dsGoose`）。`svID` 用 `SampledValueControl` 的 `smvID`，不再拼接。数据集成员按 `FCDA` 顺序。`ldInst` 不写就用控制块所在的逻辑设备。`lnClass` 不写时，该成员属于这份数据集所在的 `LLN0`（`prefix` 空、`lnInst` 空）。写了 `lnClass` 时，逻辑节点名是 `prefix`、`lnClass`、`lnInst` 直接相连。没有 `doName` 的 `FCDA` 不能编码成成员，发布退出码 2。有 `daName` 的成员是一个数据属性；没有 `daName` 时是该功能约束下的整个数据对象。写了 `ix` 时只取该数组元素，下标从 0 起；没写 `ix` 时不把数组拆成多个成员。发布时，没有 `daName` 的成员不接受一条标量 `--value`。成员类型取 `DataTypeTemplates` 里对应 DA 或 BDA 的 `bType`，不按名字猜测。没写 `--value` 的叶子按该 `bType` 的零值：布尔为假，整数和浮点为 0，字符串为空，`Quality` 为 13 位全 0（validity=good；bit2–12 细目与 test/operatorBlocked 全关），时间的秒和小数为 0。SV 按 9-2 用 13 位，不加 9-2LE 的 derived 位。布尔在 GOOSE/MMS Data 里发 1 字节：假=`0x00`，真发 `0x01`；收端按 Annex A，非 0 皆为真。`FLOAT32` 按 MMS Floating-point：格式宽度 32、指数宽度字节值为 8，后面 4 字节 IEEE 754 单精度（合计 ASN.1 内容 5 字节）；定长 GOOSE 同此（tissue 817）；NaN 合法。

GOOSE 计数按 8-1 发布状态机（Figure 10），不用 0 去填 `stNum`。`stNum` 从 1 起；这次发布不改变数据集时保持 1。`GoEna` 变为真后发出的第一帧，以及每次数据集变化后的第一帧，`sqNum` 都为 0；之后每次重传加 1，溢出时跳过 0、回到 1。数据集内容变化时 `stNum` 加 1，并把 `sqNum` 置 0。`T` 是 `stNum` 最近一次变化的 UTC 时间。GOOSE 的 `T`、控制结构里的 `T`、以及采样值里出现的时间，都是 8 字节的 UtcTime（秒 4 + 小数 3 + 品质 1）：秒为大端自 1970-01-01 UTC；小数 24 位，单位 2⁻²⁴ 秒（§8.1.3.7 把第 8 字节留给品质，不用 Annex F 里完整 32 位小数）。品质字节按 8-1：bit7 LeapSecondsKnown=0，bit6 ClockFailure=0，bit5 ClockNotSynchronized=1，bit4–0 精度=31（未指定）。秒和小数仍取操作系统时钟。`timeAllowedtoLive` 的单位是毫秒；8-1 要求至少等于下一次重传间隔，本实现取两倍。重传从这条 `GSE` 的 MinTime 开始，每次加倍，直到它自己的 MaxTime，之后保持 MaxTime；状态一变先立即发一帧。`numDatSetEntries` 等于实际编码的成员个数。地址和数据集都齐时 `ndsCom` 为假。Ed2.1 发布端必须带上 `goID`，取值就是 `GSEControl/@appID`（VisString129；tissue 1739：ASN.1 仍标 OPTIONAL 只为兼容旧发布端；本端发布不省略）；超过 129 退出码 2。PDU 内 Simulation / ndsCom 布尔同 allData：发 `0x01`/`0x00`，收端非 0 为真。Ed2.1 里 PDU 布尔量（ACSI 名 Simulation，ASN.1 旧名 test，tag [7]）与以太网头 `Reserved1` 的仿真位是同一含义，取值必须一致；命令行 `--test` 同时写这两处，默认 `true`。二者不一致的帧视为畸形，订阅丢弃。`Reserved1` / `Reserved2` 各为 2 个八位组、大端；仿真位是 `Reserved1` 的最高位（掩码 `0x8000`，Annex C Figure C.5 / Wireshark `goose.reserve1.s_bit`）；其后 3 个 R 位与 Reserved Security 在不做 62351 时均为 0。`Reserved2` 在不做 62351 时为 0。Ethertype 之后的 APDU 头顺序是 APPID（2）→ Length（2）→ Reserved1（2）→ Reserved2（2）→ IECGoosePdu / SavPdu；Length 为大端无符号，值为 `8 + m`（`m` = APDU 八位组数，且 `m < 1492`）。订阅端 Length 与实际帧长不一致、或 `< 8`，丢弃该帧，不计入 `--count`。品质属性里的 test 位是另一回事，不由 `--test` 改写。

报告是 MMS InformationReport。`variableListName` 为 VMD 作用域的 `"RPT"`。AccessResult 顺序按 8-1 Table 64：`RptID`、`OptFlds` 必有；其后按 OptFlds 才出现 `SqNum`、`TimeOfEntry`（同上 6 字节 BINARY-TIME，不是 UtcTime）、`DatSet`、`BufOvfl`、`EntryID`（固定 8 个八位组）、`ConfRev`、`SubSeqNum` 与 `MoreSegmentsFollow`（后两者同时出现或同时不出现）；然后必有 inclusion 位串（位数 = 数据集成员数；bit0=第一个成员，顺序与 NamedVariableList 一致；置位才带后续项）。其后三段各自只含 inclusion 为真的成员、且顺序相同：若 OptFlds.dataRef 则先是一组 data-reference；再是一组值；若 OptFlds.reasonCode 则再是一组 ReasonCode。ReasonCode 宽度与该 RCB 的 `TrgOps` 相同（AccessResult 条文）；位号对齐 TrgOps（bit0 保留，bit1 dchg … bit5 gi）。装置若回更长位串，多出的位（如 Ed2.1 日志侧的 application-trigger）可忽略。仅 URCB：`RptEna` 从假变真时 `SqNum` 置 0。BRCB：使能不因此把 `SqNum` 置 0，序号连续以便重连后对齐（tissue 301）；溢出后回到 1。`ResvTms`：0=未占用；正值=动态占用，关联丢失后保持该秒数；`-1`=配置独占（只读到即可，客户端不写负值）。SCL `ReportControl/@resvTms` 是布尔，只表示装置是否带该属性，不是运行时秒数。URCB 上 `OptFlds` 的 buffer-overflow 与 entryID 读回恒为假，写入忽略。`Owner` 是八位组串：占用中为客户端 IPv4 的 4 个八位组（如 `192.168.0.23` → `C0 A8 00 17`，tissue 951），不是点分文本；未占用时为空。

报告控制块的 `TrgOps` 可以不写。不写时 `gi` 按模式缺省为真，其余触发为假。SCL 里的属性名是 `period`，ACSI / MMS 里对应 integrity 位；读回或写 `TrgOps` 时按装置上的位串处理，不要把 SCL 属性名直接当成 MMS 成员名。线上 `TrgOps` 是 6 位 BIT STRING（bit0 保留）：bit1 dchg、bit2 qchg、bit3 dupd、bit4 integrity、bit5 gi。`IntgPd` 单位是毫秒；integrity 为假或 `IntgPd` 为 0 时，不发完整性报告。`ReportControl` 的 `OptFields` 元素在模式里必填；属性都没写或写成空元素时，按模式缺省：只有 `bufOvfl` 为真。线上 `OptFlds` 是 10 位 BIT STRING（bit0 保留）：bit1 seqNum、bit2 timeStamp、bit3 reasonCode、bit4 dataSet、bit5 dataRef、bit6 bufOvfl、bit7 entryID、bit8 configRef、bit9 segmentation。注意 SCL 属性书写顺序与上述位号不是同一顺序，映射时按位号，不要按 XML 属性出现次序。`rptID` 不写或为空时，报告里的标识用该报告控制块的对象引用。`indexed` 缺省为真，`RptEnabled/@max` 缺省为 1，所以装置上的名字通常是 SCL 名称加 `01`。

官方服务按这个口径是 56 个（不含已废弃的 GSSE），本项目实现 54 个；不做的是 S36、S37。下表只列客户端这几步要做的。Step 6 的行在 [PLAN_server.md](PLAN_server.md)。编号与 libiec61850 功能表的 S1–S60 一致。dchg / qchg / dupd / integrity（SCL 写作 `period`）/ GI 是报告触发条件，不是另开的服务，测报告时要覆盖。C = libiec61850，Go = iec61850ctl，R1 = `iec61850_1`。

| 编号 | 功能 | C | Go | R1 | 何时 |
|------|------|---|----|----|------|
| S1 | GetServerDirectory | 有 | 有 | 有 | Step 3。LOGICAL-DEVICE：GetNameList objectClass=DOMAIN、vmdSpecific |
| S2 | Associate | 有 | 有 | 有 | Step 2 |
| S3 | Abort | 有 | 有 | 部分 | Step 2 起能处理。ACSI 与 MMS 同名。对端或本端拆链后退出码 3 |
| S4 | Release | 有 | 有 | 部分 | Step 2 |
| S5 | GetLogicalDeviceDirectory | 有 | 有 | 有 | Step 3。GetNameList NamedVariable、domainSpecific；无 `$` 的是 LN 名 |
| S6 | GetLogicalNodeDirectory | 有 | 有 | 部分 | Step 3。同一 domain 的 NamedVariable，再按 LN/FC 过滤。`list journals` 只列 LOG；不带 `--ld` 时先 S1 再逐台问 |
| S7 | GetAllDataValues | 有 | 无 | 无 | Step 3。命令是 `get ln`。`--fc` 不写则读全部功能约束 |
| S8 | GetDataValues | 有 | 有 | 有 | Step 2 的样板，Step 3 接到命令 |
| S9 | SetDataValues | 有 | 有 | 有 | Step 3。通用写拒绝 CO |
| S10 | GetDataDirectory | 有 | 有 | 部分 | Step 3 |
| S11 | GetDataDefinition | 有 | 部分 | 部分 | Step 3。由 `list das` 和 `tree` 带出类型，不单开命令 |
| S12 | GetDataSetValues | 有 | 有 | 有 | Step 3。`get ds` 读值。只强制 `--dataset`，成员顺序与装置一致 |
| S16 | GetDataSetDirectory | 有 | 有 | 有 | Step 3。MMS GetNamedVariableListAttributes。`get ds` 先拿成员引用；`list dss` 另用 GetNameList NamedVariableList 列名 |
| S24 | Report（BRCB） | 有 | 有 | 部分 | Step 3 |
| S25–S26 | Get/SetBRCBValues | 有 | 有 | 有 | Step 3 |
| S27 | Report（URCB） | 有 | 有 | 部分 | Step 3 |
| S28–S29 | Get/SetURCBValues | 有 | 有 | 有 | Step 3 |
| S32–S34 | 按时间、按条目、日志状态 | 有 | 有或一部分 | 无 | Step 3。只由 `get journal` 发起。CSV 列为 `entry_id,time,tag,value`。默认 `--from` 为当天 00:00；有 `--entry` 则从该条之后读 |
| S35 | SendGOOSEMessage | 有 | 有 | 无 | Step 4，仅 Linux 网卡 |
| S38–S39 | Get/SetGoCBValues | 有 | 有 | 无 | Step 4 |
| S45 | SendMSVMessage | 服务端有 | 有 | 无 | Step 4，仅 Linux 网卡。这里是客户端发布 |
| S46–S47 | Get/SetMSVCBValues | 有 | 有 | 无 | Step 4 |
| S51–S55 | Select、SelectWithValue、Cancel、Operate、CommandTermination | 有 | 有或一部分 | 无 | Step 3。按 ctlModel 对照表。Operate 必须先有 `--yes`。失败为退出码 6。Cancel 在 Select 或 SelectWithValue 成功且 Operate 失败时发出。`--confirm-ref` 对不上才是 7 |

进程退出码不是 IEC 61850 条文里的号码。装置在报文里返回的 ServiceError、Abort 和 MMS 错误，由命令行收成下面这张表。表的含义对齐 `iec61850ctl`，号码就此固定，不再另选。

| 退出码 | 含义 |
|--------|------|
| 0 | 成功。`--help` 也是 0 |
| 1 | 失败，但分不到下面任何一类 |
| 2 | 用法或输入不对：旗标、引用、配置不合法，未知的 `--format`，工程文件打不开，或发布、订阅用的控制块没有可用地址。`GSE` 没有可用的重传时间，或采样模式是每周波点数，也是 2，帧不发出，stderr 用 Notes 里已写死的句子。控制或发布没带 `--yes`，以及 `--test=false` / `--simulation=false` 时没带 `--live`，同样是 2 |
| 3 | 连接失败，或连上之后断开。对端 Abort、报告订阅中途拆链，都是 3 |
| 4 | 连接或请求超时。GOOSE/SV 抓包、订阅写了 `--count` 和 `--duration`，时限内没凑满条数，也是 4。两个旗标默认都是 0，不写则一直到 Ctrl+C |
| 5 | 找不到：对象、文件、数据集、IED、报告控制块、记录本、GOOSE/SV 控制块或网卡。含装置回答的 object-non-existent，以及逻辑设备不在本次 domain 列表（`type=NOT_ON_DEVICE`） |
| 6 | 被拒绝或没有权限。含选择、执行或 CommandTermination 失败，GOOSE/SV 控制块写被拒，以及网卡原始套接字没有权限 |
| 7 | 额外读回对不上，且只在写了核对旗标时发生：`set object --verify`、`control operate --confirm-ref`，以及控制块 set 写了 `--enable` 或 `--disable` 后 GoEna/SvEna 读回与请求不一致。不写这些旗标就不做额外读回 |
| 8 | 装置不支持该服务，或当前系统做不了这件事。非 Linux 上的网卡抓包、订阅、发布是 8 |
| 130 | 被 Ctrl+C 或 SIGTERM 打断 |

某一行读失败时，value 为空、type 写原因，stdout 仍打出这一行。多行失败时进程只回一个退出码，在 8、6、5、4、3、2、1 里取数字最大的那一类，不另造号码。命令行旗标、默认值、接受范围、打印的错误原文或本表一旦改动，同一轮改 `README.md` 和 `README-zh.md`。Step 1 写这两份说明时，把本表原文放进去。

`get ds` 的官方输入是一个数据集引用，形式是 `LD/LN.数据集名`，没有功能约束，也没有「不写就用某一组」的默认值。所以只强制 `--dataset`。官方输出是按数据集里的顺序排出的成员值。标准没有规定打印格式，stdout 用和 `get object` 相同的 CSV，一行一个成员；某个成员没有品质或时间时，对应格子为 `-`。

目录服务的 MMS 映射按 8-1（tissue 621）：`list lds` / GetServerDirectory(LOGICAL-DEVICE) 发 GetNameList，objectClass=DOMAIN、objectScope=vmdSpecific；`list lns` / GetLogicalDeviceDirectory 发 GetNameList，objectClass=NamedVariable、objectScope=该逻辑设备 domain，客户端只把名字里没有 `$` 的当成 LN；带 `$` 的是数据或控制块路径，留给 `list dos` / `list reports` 等按 FC 再筛。`list dss` 发 GetNameList，objectClass=NamedVariableList。`list journals` / GetLogicalNodeDirectory(LOG) 发 GetNameList，objectClass=Journal、objectScope=该逻辑设备 domain（8-1 Table 39）。所有 GetNameList 在 `moreFollows=true` 时用上一批最后一个标识作 `continueAfter` 继续，直到 `false`。`list das` / GetDataDirectory 与 GetDataDefinition 都映射为 GetVariableAccessAttributes（8-1 §13.4.3–13.4.4），从 typeDescription 拆出子对象/属性名；GetDataSetDirectory 是 GetNamedVariableListAttributes，不是列数据集名。

`get ln` 对应官方的 GetAllDataValues（8-1 §12.3.2 / Table 41）。必须给出逻辑节点 `--ln`（`LD/LN`）。不写 `--fc` 时发对该 LN 的 MMS Read（整节点）；写了 `--fc` 时用 alternateAccess，component 为该功能约束名。打印仍用和 `get object` 相同的 CSV，一行一个数据。`iec61850ctl` 没有这条命令。GetDataDefinition 不单开命令，类型跟在 `list das` 和 `tree` 里一起给出。
