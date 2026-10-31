# iec61850-cmd 服务端

客户端见 [PLAN_client.md](PLAN_client.md)。等那边 Step 3 和 Step 4 都勾完再开始。库、退出码表和 MMS 客户端命令以客户端计划为准，这里不重写。

## Description

做一个纯 Rust 的 IEC 61850 服务端命令行 `iec61850_server`。不调用 libiec61850，也不用 CGO。验收是被 libiec61850 客户端浏览、读值，并完成一次普通报告的总召唤。不声称和标准全文或一致性实验室一致。

必须用 `--scl` 启动。根元素要在 `http://www.iec.ch/61850/2003/SCL` 里，并且至少有一台带逻辑设备的 IED。ICD、CID、SCD 是预期种类；IID、SED 只要满足这一点也接受。SSD 没有 IED，退出码 2。`version` / `revision` / `release` 不是 `2007` / `C` / `5` 时记到 stderr，不因此拒绝整份文件。不认识的工程段落记下节点名并跳过，不拒绝整份文件。JSON 只覆盖已经存在的叶子初值，不能代替工程文件。

第一刀不做：不带 `--scl` 的空服务端、导出 `.cfg`、文件服务、物理量仿真、写、控制、缓冲报告。这些留在 Step 6。S36、S37 不实现。S40–S44 GSSE 不实现、不计数。对时用操作系统时钟。

## Steps

### Step 5: 服务端第一刀

等 [PLAN_client.md](PLAN_client.md) 的 Step 3 和 Step 4 都勾完再开始。

- [ ] 从 SCL 读出 LD、LN、DO、DA、数据集、报告。逻辑设备名见 Notes。不认识的节点记下并跳过，其余照常
- [ ] `iec61850_server start --scl` 接受 Associate（MMS Initiate），并能 Release（MMS Conclude）与 Abort。Initiate 响应：`negotiatedVersionNumber` 固定为 Integer16 `1`；PDU / outstanding / 嵌套按双方取小回给客户端；带回 `negotiatedParameterCBB` 与 `servicesSupportedCalled`（覆盖本端实际支持的服务）。本端收发也用谈妥值。请求版本不是 1 则回 Initiate 错误，不建关联。正常断开走 Conclude；异常才 Abort。可选 `--values` 只覆盖已有叶子初值，形状见 Notes。写了 `--ready-json` 时，监听成功后向 stdout 打一行就绪 JSON；人看的监听提示仍在 stderr
- [ ] libiec61850 客户端能浏览、读到一个值，并完成一次 URCB 总召唤。浏览至少应答 GetServerDirectory、GetLogicalDeviceDirectory、GetLogicalNodeDirectory、GetDataDirectory；读值应答 GetDataValues；报告控制块应答 GetURCBValues / SetURCBValues。`TrgOps` 的 `gi` 为真时，写 GI 就用 Report（MMS InformationReport）发出一次总召唤；`gi` 为假时拒绝这次写，不发 Report。这是拒绝，不是「不支持」。`RptEna=true` 时，除 `RptEna` 和 `GI` 外，其它报告控制块属性的写一律拒绝。URCB 的 `Resv` 只能在未使能时写；同一实例同时只允许一个客户端占用，后来的 `Resv=true` 拒绝。`Resv=true` 时填写 `Owner`（可选属性，有则回给 GetURCBValues）：值为该客户端 IPv4 的 4 个八位组，不是点分文本；未占用时为空八位组串。客户端须先成功写 `Resv=true` 再配置或使能（tissue 1432）；第一刀若客户端未先 Resv 就写 `RptEna`，本端可代为占用，但仍只允许一个客户端。该关联 Conclude、Abort 或 TCP 拆链后，清掉它占用的 `Resv`/`Owner`，并使能的 URCB 退出使能。仅 URCB：`RptEna` 从假变真时 `SqNum` 置 0，按无符号 8 位应答。URCB 的 `OptFlds` 里 buffer-overflow 与 entryID 恒为假。总召唤报告的 InformationReport 使用 VMD 名 `"RPT"`，AccessResult 顺序同客户端计划里的 Table 64；`TimeOfEntry` 若出现则为 6 字节 BINARY-TIME（4 字节毫秒 + 2 字节自 1984-01-01 起的天数）。
- [ ] 解析并保存 `<Services>`。元素缺了就是没声明，回「不支持」，与 S36、S37 相同。元素名用模式里的名字，对照见 Notes。第一刀的验收文件要声明 `GetDirectory`、`DataObjectDirectory`、`ReadWrite`、`GetCBValues`，并且至少有一个 `buffered` 不为真的报告控制块
- [ ] 不做不带 `--scl` 的空服务端。不做导出 `.cfg`。本步不做文件服务。收到 SIGINT 或 SIGTERM 时干净退出，退出码 130

### Step 6: 服务端其余半边，以及不挡第一刀的库能力

- [ ] 服务端的写、控制、缓冲报告，以及其余服务端半边。控制按客户端计划里的 ctlModel 对照表应答；status-only 拒绝执行。Select 应答对 `$SBO` 的读：肯定优先回该控制对象的 `CO_CtrlObjectRef` Visible-string（tissue 1753 / 1178）；否定回空串（仍是读成功）。客户端侧须接受任意非空。SelectWithValue / Operate / Cancel 分别应答对 `$SBOw` / `$Oper` / `$Cancel` 的写。除 `$SBO` 外，其它 `FC=CO` 的读拒绝（tissue 1288）。Operate / SelectWithValue 缺了 `ctlVal`、`origin`、`ctlNum`、`T`、`Test` 或 `Check` 时拒绝该写；Cancel 缺了 `ctlVal`、`origin`、`ctlNum`、`T` 或 `Test` 时拒绝（Cancel 无 `Check`）。增强安全下：CommandTermination 肯定结束发该控制 `$Oper` 的 InformationReport；否定结束发 InformationReport，先 VMD `"LastApplError"`（`CntrlObj`、`Error`、`Origin`、`ctlNum`、`AddCause`），再跟 `$Oper`（8-1 Table 92）。选择或执行被拒时同样可先发 `LastApplError` 再回否定写应答。BRCB：`EntryID` 固定 8 个八位组；`RptEna` 假→真不把 `SqNum` 置 0（tissue 301），按无符号 16 位应答，溢出回到 1。对 `RptID`、`DatSet`、`BufTm`、`TrgOps`、`IntgPd` 或 `PurgeBuf` 的成功改写须执行 7-2 的清缓冲过程（8-1 §17.1.2）。`ResvTms` 语义同客户端计划（0 / 正秒数 / `-1` 配置独占）
- [ ] 库里补上定值组、取代（FC=SV，Substitution）、单播采样、定时执行、文件传输。这些没有第一版子命令。`--files-dir` 在这一步才接上
- [ ] 本项目客户端与本项目服务端对打，再用 libiec61850 客户端打回来
- [ ] S36、S37 不实现。服务端收到时回「不支持」

### Notes

版本口径与 [PLAN_client.md](PLAN_client.md) 相同：6 是 Edition 2.2，XML 命名空间是 `http://www.iec.ch/61850/2003/SCL`，模式版本是 2007C5。7-2 / 7-3 / 7-4 / 8-1 / 9-2 的现行合并版是 Edition 2.1。SCL 模式已在本地，其余部分的 PDF 全文仍不在仓库里。SCL 在标准里是工程交换文件；本项目的服务端必须 `--scl` 是产品决定。监听的 IP 和端口用 `--host` 和 `--port`，不用工程文件里的 IP 替换本机绑定地址。`OSI-TSEL`、`OSI-SSEL`、`OSI-PSEL` 取自选定 IED 的 `ConnectedAP/Address`，按十六进制字节原样使用。`--port` 不写时，用文件里的 `MMS-Port`；文件里也没有，才用 102。选择子缺了任意一项时，缺的那项用 `0001` / `0001` / `00000001`。

客户端和服务端只在关联角色和分发方向上分叉。服务处理只调用 path、data、dispatch，不碰 TCP。报告是 ACSI 的 Report，在 MMS 里叫 InformationReport，不走确认分发。

| 项 | 决定 |
|----|------|
| 顺序 | 客户端 Step 3 和 Step 4 完成后再做本文件 |
| 工程文件 | 必须 `--scl`。预期 ICD/CID/SCD；IID、SED 有带逻辑设备的 IED 时也接受。SSD 退出码 2。可选 `--values` 的 JSON 只覆盖叶子初值 |
| 不认识的段落 | stderr 记下节点名并跳过，不拒绝整份文件 |
| 解析深度 | LD/LN/DO/DA、数据集、报告、基本 CDC。LCB 可先存着 |
| 报告 | 第一刀只做 URCB：Get/SetURCBValues。`gi` 为真时，使能后写 GI 发一次 Report；`gi` 为假时拒绝写 GI。缓冲报告在 Step 6 |
| 服务端连接 | 默认同时 5 个，可用 `--max-connections` 改。值在内存里 |
| 传输 | 明文 TCP。TLS 只留配置口，不验收 |
| `<Services>` | 解析并保存。没声明的操作回「不支持」。不依赖第三方库来做这件事 |
| 不做 | 空服务端、导出 `.cfg`、第一刀的文件服务、物理量仿真、S36、S37、GSSE |

`iec61850_server start --scl` 的可选旗标：`--ied-name`（默认第一台带逻辑设备的 IED）、`--values`、`--host`（默认 `0.0.0.0`）、`--port`（默认 `102`，`0` 表示系统选端口）、`--max-connections`（默认 5）、`--ready-json`。`--files-dir` 留到 Step 6。

IEC 61850 不规定命令行怎么打印，也不规定用 JSON 文件灌初值。下面两处 JSON 是产品约定，对齐 `iec61850ctl`，不要求逐字段相同。

**`--values`：** 工程结构仍来自 `--scl`。文件是一个对象，只覆盖已经存在的叶子，不能凭空造出模型。不认识的叶子记到 stderr 并跳过。文件打不开或 JSON 不是这个形状，退出码 2，服务端不启动。

```json
{
  "Leaves": [
    {"Ref": "HV_3_AH_04CTRL/BIGGIO1.SPCSO10.stVal", "FC": "ST", "Type": "BOOL", "Value": {"Raw": false}}
  ]
}
```

`Leaves` 必填。每一项必填 `Ref`（`LD/LN.DO.DA`）、`FC`、`Type`、`Value.Raw`。`Type` 用大写短名 `BOOL`、`INT`、`UINT`、`FLOAT`、`ENUM`、`STRING`。`LogicalDevices` 可空，可省略。

**`--ready-json`：** 监听成功后向 stdout 打一行 JSON，至少含 `event=ready`、`address`、`ied_name`、`adapter=iec61850_server`、`version`。人看的「正在监听」仍打在 stderr。自动化读这一行再去连。`--port 0` 时以这一行里的地址为准。

服务端进程自己的退出码只用客户端表里已经有的号码：参数、工程文件或 `--values` 不合法为 2；收到 SIGINT 或 SIGTERM 为 130。正常监听不退出。客户端命令的 0–8、130 全表在 [PLAN_client.md](PLAN_client.md)。

第一刀要能应答：GetServerDirectory、GetLogicalDeviceDirectory、GetLogicalNodeDirectory、GetDataDirectory、GetDataValues、GetURCBValues、SetURCBValues，以及 `gi` 为真时由 GI 触发的 Report。目录服务按 8-1 映射到 MMS `GetNameList`：GetServerDirectory(LOGICAL-DEVICE) 用 objectClass=DOMAIN、vmdSpecific，列出逻辑设备名；GetLogicalDeviceDirectory / GetLogicalNodeDirectory 用 objectClass=NamedVariable、domainSpecific，返回的标识可含带 `$` 的路径，由客户端筛 LN（无 `$`）或按 FC 筛 DO/控制块；`moreFollows` 为真时接受 `continueAfter` 继续。GetDataDirectory 列出数据属性名。数据集成员目录是 GetNamedVariableListAttributes，不是 GetNameList。`<Services>` 里的元素名不是 ACSI 服务名。某个元素没出现就是没声明，该组操作回「不支持」。第一刀的验收文件要有 `GetDirectory`、`DataObjectDirectory`、`ReadWrite`、`GetCBValues`，并且至少有一个非缓冲报告控制块。Step 6 才做下表。官方服务一共 56 个（不含已废弃的 GSSE），本项目实现 54 个；不做的是 S36、S37。

逻辑设备名与客户端相同：有 `ldName` 用 `ldName`，否则是 IED 名接 `inst`。MMS 域名用这个逻辑设备名。`ReportControl` 的 `indexed` 缺省为真，`RptEnabled/@max` 缺省为 1。为真时实例名是 SCL 名称加两位序号，从 `01` 排到 `max`。为假时就用 SCL 里的名字。`buffered` 缺省为假，第一刀只应答这种 URCB；`buffered="true"` 留到 Step 6。`TrgOps` 不写时 `gi` 为真。SCL 的 `period` 对应装置上的 integrity 位。线上位号与客户端计划相同：TrgOps bit1–5 = dchg/qchg/dupd/integrity/gi；OptFlds bit1–9 按 8-1 Table 38，不要按 SCL 属性书写顺序。`IntgPd` 为 0 或 integrity 未置位时，第一刀不发完整性报告。`OptFields` 元素在模式里必填；属性都没写或空元素时，只有 `bufOvfl`（bit6）为真，其余可选字段为假；发出的 InformationReport 只带这些为真的可选字段。`rptID` 不写或为空时，报告标识用该 URCB 的对象引用，形式是 `{逻辑设备名}/{LN}$RP${实例名}`。

| `<Services>` 元素 | 没出现时怎么处理 |
|-------------------|-------------------|
| `GetDirectory` | 目录服务回「不支持」，含 GetServerDirectory、GetLogicalDeviceDirectory、GetLogicalNodeDirectory |
| `DataObjectDirectory` | GetDataDirectory 回「不支持」 |
| `GetDataObjectDefinition` | GetDataDefinition 回「不支持」 |
| `ReadWrite` | GetDataValues、SetDataValues 回「不支持」 |
| `GetDataSetValue` | GetDataSetValues 回「不支持」 |
| `SetDataSetValue` | SetDataSetValues 回「不支持」 |
| `DataSetDirectory` | GetDataSetDirectory 回「不支持」 |
| `DynDataSet` | CreateDataSet、DeleteDataSet 回「不支持」 |
| `GetCBValues` | 读报告、日志、GOOSE、SV 控制块回「不支持」 |
| `ConfReportControl` | 不能在线新增报告控制块。SCL 里已经有的 URCB 仍要能被使能并做总召唤 |
| `ConfLogControl` | 不能在线新增日志控制块。本项目不打开 LogEna |
| `TimerActivatedControl` | TimeActivatedOperate 回「不支持」 |
| `SettingGroups` | 定值组回「不支持」 |
| `SupSubscription` | 取代回「不支持」 |
| `FileHandling` | 文件传输回「不支持」。`ftp` / `ftps` 缺省为假，本项目也不做 |
| `GOOSE` | 不发布 GOOSE。元素在时，`goose` 缺省为真，`rGOOSE` 缺省为假；可路由 GOOSE 回「不支持」 |
| `SMVsc` | 不做采样值控制块服务。元素在时，`delivery` 缺省是 `multicast`，`sv` 缺省为真，`rSV` 缺省为假 |
| `GSEDir` | 不拿它来拒绝其它服务。S36、S37 无论有没有这个元素都回「不支持」 |
| `GSSE` | 不实现。出现了也回「不支持」 |

`ReportSettings`、`GSESettings`、`SMVSettings` 里的属性缺省是 `Fix`。标成 `Conf` 或 `Fix` 的项，在线修改回「不支持」。`RptEna`、GI、`GoEna`、`SvEna` 不在这些属性里，按控制块操作处理。`ReportSettings` 整段都没有时，不因此拒绝已有 URCB 的使能和总召唤。

| 编号 | 功能 | 何时 |
|------|------|------|
| S13 | SetDataSetValues | Step 6。无第一版子命令 |
| S14 | CreateDataSet | Step 6。无第一版子命令 |
| S15 | DeleteDataSet | Step 6。无第一版子命令 |
| S17 | 取代 | Step 6。无第一版子命令。功能约束是 SV（Substitution），不是采样值 |
| S18–S23 | 定值组 | Step 6。无第一版子命令 |
| S30–S31 | Get/SetLCBValues | Step 6。不打开 LogEna |
| S36–S37 | GetGoReference、GetGOOSEElementNumber | 不做。收到时回「不支持」 |
| S48–S50 | 单播采样 | Step 6。无第一版子命令 |
| S56 | TimeActivatedOperate | Step 6。无第一版子命令 |
| S57–S60、F1 | 读文件、上传、删除、属性、目录 | Step 6。第一版无子命令。`--files-dir` 在这一步 |
