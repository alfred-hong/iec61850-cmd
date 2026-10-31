# IEC 61850 官方核对（对照 PLAN）

日期：2026-10-29。结论已写回 `PLAN_client.md` 和 `PLAN_server.md` 的 Notes。

## 用了什么

- IEC Webstore：`IEC 61850:2026 SER`（2026-10-14）、`IEC 61850-7-2:2010+AMD1:2020 CSV`、`IEC 61850-8-1:2011+AMD1:2020 CSV` 的官方范围说明。
- IEC 61850-6 Edition 2.2 官方预览里的范围、命名空间 2007C5、code component 分发说明。
- 仓库 `1MRK511393-UEN` Rev. N（Hitachi Energy，2025-03）：产品通信手册，不是 IEC 正文。
- 仓库 `iec61850-intro.pdf`：2009 年讲义，未当作标准。

标准 PDF 全文未取得，条款级措辞以官方摘要、预览和产品手册中的标准引用为限。

## 与原 PLAN 不一致、已改的点

1. 版本不能笼统叫 Edition 2。6 为 Edition 2.2；7-2、7-3、7-4、8-1、9-2 的现行合并版为 Edition 2.1。
2. GOOSE 属于 8-1 的以太网映射，站级总线和过程层都可以用。
3. 9-2LE 不是 IEC 标准。
4. GetNameList / Read / InformationReport 是 MMS 服务名；ACSI 名在 7-2。
5. 禁止一切 FC=CO 写入与控制映射冲突。通用写命令拒绝 CO；控制命令必须写 CO。
6. ctlModel 是五个安全模型；SBOw 不是 ctlModel 名。
7. SCL 是工程交换文件。强制 `--scl` 是产品决定。`<Services>` 被忽略时，不能声称与该 SCL 的服务声明一致。
8. 一致性测试是 61850-10。安全是 IEC 62351。8-2（XMPP）是另一条 SCSM，本项目不做。

## 拆分后再核对

标准 PDF 全文仍不在仓库里。对照 7-2 的 ctlModel 和 8-1 的关联服务名，又改了这些：

1. `status-only` 不能执行控制。`direct-with-enhanced-security` 不先选择，执行后要等 CommandTermination。不能把五个模型说成「走同一套状态机」。
2. S2 的 ACSI 名是 Associate，MMS 名是 Initiate。S4 的 ACSI 名是 Release，MMS 名是 Conclude。
3. `--mode` 短名与 ctlModel 的对应写成表：`direct` 对两个 direct 模型；`sbo` 只对 sbo-with-normal-security（发 Select）；`sbow` 只对 sbo-with-enhanced-security（发 SelectWithValue）。SBOw 仍是对象名。
4. Cancel 在 Select 或 SelectWithValue 成功、Operate 失败时发出。`--confirm-ref` 不是 CommandTermination。
5. Report 的 MMS 名是 InformationReport。Abort 两边同名。
6. 服务端第一刀的「浏览」要能应答 GetServerDirectory、GetLogicalDeviceDirectory、GetLogicalNodeDirectory、GetDataDirectory，不只是列逻辑设备。
7. 服务端第一刀的 URCB 总召唤还要 Get/SetURCBValues，并使能后写 GI 时发出 Report（InformationReport）。
8. 取代的功能约束 SV 是 Substitution，不是采样值 Sampled Values。

## 对照本地 2007C5 之后

本地文件是 `standards/iec-free-code-components/IEC_61850-6.SCL.2007C5.full/`。网上现行出版物仍是 IEC 61850-6:2009+AMD1:2018+AMD2:2024 CSV（Edition 2.2，代码包名 `IEC_61850-6.SCL.2007C5.Full`），以及 7-2 / 8-1 的 Edition 2.1 合并版。7-2、8-1、9-2 的 PDF 全文仍不在仓库里。已删的委员会草案不作为依据。

按模式改过的点：

1. XML 命名空间是 `http://www.iec.ch/61850/2003/SCL`。`2007C5` 是模式版本和命名空间名字，不是 xmlns。根属性固定 `2007` / `C` / `5`；对不上时记 stderr，不拒绝整份文件。
2. 目的 MAC、十六进制 APPID、VLAN、MinTime、MaxTime 在 `GSE` / `SMV`，用 `ldInst` 与 `cbName` 对到控制块。`GSEControl/@appID` 是 goID 字符串。
3. APPID 按十六进制读。GOOSE Type 1 为 `0x0001`–`0x3FFF`、Type 1A 为 `0x8000`–`0xBFFF`（`0x0000` 未配置），SV 为 `0x4000`–`0x7FFF`（第十六批纠正）。MinTime / MaxTime 的数字是毫秒。后来改为缺了就不发布。`VLAN-ID` 为 `000` 时仍打标签，见第八批。
4. `smpMod` 缺省 `SmpPerPeriod`。模式里没有额定频率。后来改为这种文件不发布，不再乘 50 Hz，见下文。
5. `type="GSSE"`、`Protocol` 为 R-GOOSE / R-SV、或 `multicast="false"` 时不发以太网帧，退出码 8。
6. `<Services>` 用模式里的元素名。`ConfReportControl` 没出现时，不拒绝 SCL 里已经有的 URCB 使能和总召唤。`ReportSettings` 等属性缺省 `Fix`。
7. 服务端接受带逻辑设备的 IID、SED。SSD 退出码 2。MMS 选择子后来改为优先用工程文件里的字节，见下文。

## 再对照模式里的名字和缺省值

1. 逻辑设备名：有 `ldName` 用 `ldName`，否则是 IED 名接 `inst`，中间无分隔符。MMS 域名用这个名字。
2. `gocbRef` 拼成 `{逻辑设备名}/LLN0$GO${控制块名}`，datSet 引用拼成 `{逻辑设备名}/LLN0${datSet}`。`svID` 用 `smvID`。同一控制块对上多条 `GSE` 或 `SMV`，或发布时没有 `datSet`，退出码 2。
3. 数据集成员按 `FCDA` 顺序。没有 `daName` 的成员是整个 DO，不用一条标量覆盖。
4. `TrgOps` 不写时只有 `gi` 为真。客户端 `gi` 为假时不做总召唤。服务端此时写 GI 回「不支持」。
5. `indexed` 缺省为真，`max` 缺省为 1，实例名带 `01`。第一刀只应答非缓冲报告。验收 SCL 还要有 `DataObjectDirectory`。

## 通信段必须带上 IED

官方示例 SCD 里至少有两台 IED 的 `LDevice/@inst` 都是 `C1`。`GSE` / `SMV` 的 `ldInst` 对的是这个 `inst`，不是 `ldName`，并且它们挂在带 `iedName` 的 `ConnectedAP` 下面。只按 `ldInst`+`cbName` 会把不同 IED 配错。计划改为先用 `--ied-name` 选定 IED，不写且只有一台含该控制块时用那一台，多于一台退出码 2。

## 帧计数与总召唤拒绝

8-1 发布状态机写明：`stNum` 从 1 起；`GoEna=true` 后的第一帧以及状态变化后的第一帧 `sqNum` 为 0；重传加 1，溢出跳过 0 回到 1。`timeAllowedtoLive` 用毫秒，至少等于下一次重传间隔（实现取两倍）。`stNum` 不能用 0 填充。`gi` 为假时写 GI 是拒绝，不是服务不支持，避免被收成退出码 8。

## 去掉和帧上对不上的默认数

1. GOOSE 以太网类型 `0x88B8`，采样值 `0x88BA`。抓包、订阅、发布都按这个过滤或发送。
2. MinTime、MaxTime 只读这条 `GSE` 自己的数字。缺了或 MinTime 大于 MaxTime 时不发布，不再填入示例里另一条的 4 ms / 1000 ms。
3. `SmpPerPeriod` 不再乘 50 Hz。模式里没有额定频率，这种文件不发布。`SmpPerSec` 和 `SecPerSmp` 仍按属性本身换算。
4. 带了工程文件时，MMS 的 `OSI-TSEL`、`OSI-SSEL`、`OSI-PSEL`、`MMS-Port` 用 `ConnectedAP/Address` 里的字节。没有这些项时才用 `0001` / `0001` / `00000001` 和端口 102。`--host` 与 `--port` 仍覆盖 IP 和端口。

## 连续五轮

1. Operate / SelectWithValue 补上 7-2 的 `ctlVal`、`origin`、`ctlNum`、`T`、`Test`、`Check`。`orCat` 缺省 `remote-control`。`ctlNum` 从 1 起，同一次控制共用。普通安全的 Select 不带 `ctlVal`。
2. 数据集成员的类型改从 `DataTypeTemplates` 的 `bType` 取。`Quality` 的零值是 13 位全 0。`FCDA/@ix` 从 0 起；没写 `ix` 不拆数组。
3. UtcTime 最后一字节标明时钟未同步、精度未指定，避免把操作系统时钟说成已经同步。
4. URCB 先 `Resv=true` 再使能，退出时先取消使能再 `Resv=false`。BRCB 仍不改未写的 `ResvTms`。
5. 等 CommandTermination 使用 `--timeout`，超时退出码 4。选择已经成功时仍发 Cancel。

## 继续消除线上不一致

1. 目的 MAC：必须是组播。GOOSE 落在 `01-0C-CD-01-00-00`–`01-0C-CD-01-01-FF`，SV 落在 `01-0C-CD-04-00-00`–`01-0C-CD-04-01-FF`。
2. 有 VLAN-ID 但没有优先级时用 4，与示例 SCD 和工程手册一致。
3. 订阅报告不改写工程已配好的 DatSet/OptFlds/TrgOps 等；使能后只写 RptEna 与 GI。URCB SqNum 按 8 位，BRCB 按 16 位。
4. Cancel 带 origin/ctlNum/T/Test；SelectWithValue 之后的 Cancel 还带 ctlVal。增强安全失败时收发 LastApplError。
5. 服务端在 RptEna=true 时拒绝写除 RptEna、GI 以外的报告控制块属性。

## 继续消除线上不一致（第二批）

1. `FCDA` 缺 `lnClass` 时成员属于数据集所在的 `LLN0`。
2. `OptFields` 空元素时只有 `bufOvfl` 为真；`rptID` 空时报告标识用控制块对象引用。
3. GOOSE 发布：PDU Simulation（tag [7]）与以太网 `Reserved1` 仿真位同值，由 `--test` 写入；`Reserved2` 在无 62351 时为 0。订阅对仿真位需要 `--accept-simulation`；`ndsCom=true` 的帧不计入 `--count`。（第五批纠正了原先「simulation 固定为假」的写法。）
4. GoCB/MSVCB：`GoEna`/`SvEna` 随时可写；标识和数据集只在未使能时写；不写 `ConfRev`、`NdsCom`。
5. 服务端目录服务按 8-1 映射到 MMS `GetNameList`。
6. SCL `TrgOps/@period` 对应 ACSI/MMS 的 integrity；`IntgPd=0` 或不置位时不发完整性报告。
7. 控制 `Check` 按宽度 2 的位串发。日志 `TimeOfEntry` 线上是 Binary Time，CSV 再印成 RFC 3339。

## 继续消除线上不一致（第三批）

1. QueryLogByTime 与 QueryLogAfter 分开。After 必须带 EntryID 和 TimeOfEntry；`moreFollows` 时 stderr 同时给出 entry 与 time。
2. `list journals` 明确走 GetLogicalNodeDirectory 的 LOG。
3. URCB 占用互斥；`Resv=true` 时填 Owner。BRCB `--entry-id` 要求偶数字节十六进制。
4. Initiate 按响应谈妥的 PDU/outstanding/嵌套收发；正常结束用 Conclude，不用 Abort 代替。

## 继续消除线上不一致（第四批）

对照 BS EN 61850-8-1:2011+A1:2020 样章 Figure 10、9-2 样章 ASDU 表、本地 2007C5：

1. 纠正此前错误：使能后第一帧 `sqNum` 为 **0**（不是 1）；状态变化后第一帧也是 0；重传递增，溢出跳过 0。
2. `timeAllowedtoLive`：标准要求 ≥ 下一次重传间隔；计划写明实现取 2×。
3. `smpRate` / `nofASDU` 在模式里必填，缺则发布退出码 2，不再默认为 1。
4. 关联结束（Conclude / Abort / 拆链）清除该客户端占用的 URCB `Resv`/`Owner` 并使能失效。

## 继续消除线上不一致（第五批）

对照 8-1 Table 88 / Figure 10、tissue 1739、Ed2 仿真位说明：

1. PDU tag [7]（Simulation，旧名 test）与 `Reserved1` 仿真位必须同值；`--test` 同时写两者。禁止再写「simulation 固定为假、另写 Test」。
2. 默认 `--test=true` 时订阅要 `--accept-simulation`；品质里的 test 位不挡订阅。
3. Ed2.1 发布端必须携带 `goID`（= `GSEControl/@appID`）；ASN.1 OPTIONAL 只为收旧帧。收端缺 `goID` 时按默认 GoID（= `gocbRef`）比对。

## 继续消除线上不一致（第六批）

对照 tissue 1345 / 1725、2007C5 `tSampledValueControl`：

1. `fixedOffs` 缺省假 → 最短 BER；为真 → Annex A 定长发布。订阅按带长度的 BER 解。
2. `SmvOpts` 元素必填；缺则发布退出码 2。SV `--simulation` 只动 `Reserved1`。

## 继续消除线上不一致（第七批）

对照 8-1 Table 38/39、UtcTime 品质字节、CheckConditions、7-3 Quality：

1. 写明 `TrgOps` / `OptFlds` 的 MMS 位号（bit0 保留）；SCL 属性顺序 ≠ 位号。
2. `Check`：bit0 synchrocheck、bit1 interlock-check。
3. UtcTime 品质字节：LeapSecondsKnown=0、ClockFailure=0、ClockNotSynchronized=1、accuracy=31。
4. `Quality` 13 位全 0 = good；SV 不加 9-2LE derived。

## 继续消除线上不一致（第八批）

对照 2007C5 `tP_VLAN-*`、`tFCDA`、tissue 1791：

1. `confRev` 在 GOOSE / SV 上正文必填。缺了不再填 0，发布退出码 2。
2. `VLAN-ID` 写了就打 802.1Q，包括 `000`。没写时不打标签的写法已由第十一批按 Annex C 改为仍打标签（VID 0、优先级 4）。
3. `datSet` 必须对上同一 `LLN0` 的数据集名；`FCDA` 没有 `doName` 则不能编码。

## 继续消除线上不一致（第九批）

对照 8-1 Table 34 / §11 / §14.3.5、tissue 621：

1. GetServerDirectory → GetNameList DOMAIN + vmdSpecific；分页用 continueAfter 直到 moreFollows=false。
2. GetLogicalDeviceDirectory → NamedVariable + domain；客户端筛无 `$` 为 LN。
3. `list dss` → GetNameList NamedVariableList；GetDataSetDirectory → GetNamedVariableListAttributes；`get ds` 读值走 GetDataSetValues。
4. `OptFields` 元素模式必填；`tMessageID`（goID/smvID 等）最长 129。

## 继续消除线上不一致（第十批）

对照 8-1 控制映射与 tissue 1288：

1. Select = 读 `$CO$…$SBO`；SelectWithValue = 写 `$SBOw`；Operate = 写 `$Oper`；Cancel = 写 `$Cancel`。
2. 除 Select 外不对 `FC=CO` 发 GetDataValues。

## 继续消除线上不一致（第十一批）

对照 8-1 Table 64、§17.2.1、Table 63、Annex C Table C.1：

1. 报告：`variableListName` = VMD `"RPT"`；AccessResult 固定顺序；inclusion 必有；`TimeOfEntry` 为 6 字节 Binary Time。
2. URCB：`RptEna` 假→真时 `SqNum` 置 0；OptFlds 的 buffer-overflow 与 entryID 恒为假。
3. 纠正第八批：没写 VLAN 时发布端仍打标签，VID 0、优先级 4。订阅端无标签也收。
4. `FLOAT32` 为 MMS Floating-point（指数宽度 8 + IEEE 754 单精度）。

## 继续消除线上不一致（第十二批）

对照 8-1 §8.1.3.8 / Annex E、§17.1.2、§20.11 Table 96、tissue 951 / 1432：

1. BINARY-TIME 6 字节：毫秒（4，大端）+ 自 1984-01-01 起的天数（2，大端）。
2. `Owner`：占用中为 IPv4 四八位组（如 `C0A80017`），不是点分文本；空表示未占用。
3. URCB：客户端先 `Resv=true` 再配置/使能；服务端可为兼容代为占用，但仍互斥。
4. `LastApplError`：VMD 名 `"LastApplError"`，成员顺序 CntrlObj / Error / Origin / ctlNum / AddCause。

## 继续消除线上不一致（第十三批 / 终核）

对照 8-1 §8.1.3.5 / Table 61 / Table 92 / goID 长度、tissue 300 / 301 / 817、iec61850ctl 现场帧：

1. `EntryID` 固定 8 个八位组（报告与日志）；不再写「常用 8 字节 / 偶数十六进制」。全 0 为保留值。
2. 仅 URCB 在 `RptEna` 假→真时把 `SqNum` 置 0；BRCB 使能不复位（tissue 301），溢出回到 1。
3. `FLOAT32` 定长 GOOSE 仍是 ASN.1 长度 5（指数宽度字节 8 + IEEE 754），不是 4（tissue 817）。
4. CommandTermination：肯定结束 = `$Oper` 的 InformationReport；否定 = `LastApplError` 再跟 `$Oper`（Table 92）。
5. `--resv-tms`：INT16 秒；客户端只写 ≥0；`-1` 为配置独占（只读）；SCL `@resvTms` 布尔 ≠ 运行时值（tissue 300）。
6. 纠正 datSet 引用：必须是 `{LD}/LLN0$` + 数据集名（中间有 `$`），不是 `LLN0` 直接拼名。
7. GOOSE PDU `goID` 最长 65；SCL `appID` 可到 129，超 65 发布退出码 2。SV `svID` 仍可到 129。
8. Select 否定 = 读成功但 `$SBO` 为空串；Cancel **必带** `ctlVal`，且无 `Check`；Oper/SBOw 成员顺序固定。
9. ReasonCode ≥7 位（含 bit6 application-trigger），不只是「与 TrgOps 相同」。
10. `list journals` → GetNameList objectClass=Journal。
11. BRCB：改写 `RptID`/`DatSet`/`BufTm`/`TrgOps`/`IntgPd`/`PurgeBuf` 须走清缓冲过程。

## 继续消除线上不一致（第十四批）

对照 BS EN 61850-8-1:2011+A1:2020（Ed2.1）样章、§8.1.3.7 / Table 64、Annex A、tissue 1753 / 232：

1. 纠正第十三批：`goID` 在 **Ed2.1** 为 VisString129（最长 129），不是旧版 65。超 129 才退出码 2。
2. GOOSE/MMS 布尔：发 `0x01`/`0x00`；收端非 0 为真（Annex A）。
3. UtcTime：秒 4 + 小数 24 位（2⁻²⁴ 秒）+ 品质 1；不用 Annex F 完整 32 位小数占满第 8 字节。
4. Select+：服务端优先回 `CO_CtrlObjectRef`；客户端接受任意非空（tissue 1753）。
5. 报告 inclusion 后三段分批（data-ref 组 → 值组 → ReasonCode 组）；ReasonCode 宽度对齐该 RCB 的 TrgOps（纠正「固定 ≥7 位」）。
6. 纠正「目录一律 GetNameList」：GetDataDirectory / GetDataDefinition → GetVariableAccessAttributes。
7. GetDataSetValues 的 MMS Read 须 `specificationWithResult=TRUE`（Table 45）。

## 继续消除线上不一致（第十五批）

对照 8-1 §18.1.1 Table 73–74、§12.3.2、2007C5 `tDurationInMilliSec` / `tP_MAC-Address`、9-2 smpSynch、tissue 1582：

1. GoCB：`GoID` 缺省 = 控制块对象引用；CLI 不写 ConfRev/NdsCom/DstAddress/MinTime/MaxTime/FixedOffs；`DatSet` 写入（含同值）会使 ConfRev+1。
2. MinTime/MaxTime：内容为毫秒；`unit`/`multiplier` 可省略，写了必须是 `s`/`m`。
3. MAC：模式为大写+连字符；实现另接受 `:`/小写（产品放宽）。
4. `get ln` / GetAllDataValues：无 FC 读整 LN；有 FC 用 alternateAccess component=FC。
5. `smpSynch=0` 明确为 9-2「未同步」，不是笼统的「不做 9-3」。

## 继续消除线上不一致（第十六批）

对照 8-1 Annex C（Length / Reserved1 Figure C.5 / Table C.1–C.2）、Table 109 Initiate、现场 PICS / 互通现象：

1. GOOSE APPID：Type 1 可用 `0x0001`–`0x3FFF`，Type 1A（Trip）`0x8000`–`0xBFFF`；`0x0000` 表示未配置，不算可用地址（纠正原先只写 `0x0000`–`0x3FFF`）。
2. Ethertype PDU：顺序 APPID→Length→Reserved1→Reserved2→APDU；Length=`8+m`（`m<1492`）；不一致或 `<8` 的帧订阅丢弃。
3. `Reserved1` 仿真位 = 最高位掩码 `0x8000`；后 3 个 R 位与 Reserved Security 在无 62351 时为 0。VLAN 缺省仍打标签，TPID=`0x8100`，VID 0、优先级 4。
4. Initiate：`proposedVersionNumber` / `negotiatedVersionNumber` 线上为 Integer16 `1`（与 PICS、libiec61850 现象一致；样章 Table 109 有处 OCR/合并版写 “2.1”，按 Integer 1 互通）。须带 parameterCBB 与 servicesSupported。

## 继续消除线上不一致（第十七批）

1. Initiate：`proposedVersionNumber`/`negotiatedVersionNumber` 明确为 Integer16 `1`（PICS / 互通现象）。
2. GoCB：`GoID`/`DatSet` 按 7-2 在 `GoEna=FALSE` 时可写；注明 8-1 Table 73 标 `r` 与 7-2/tissue 1582 冲突时以 7-2 与互通为准。
3. `tMessageID` 模式 maxLength=129，与 Ed2.1 VisString129 / GoID 上限一致（已对齐）。

## 本轮结论

第十七批后再对照 Annex C、Table 64/73/109、2007C5、tissue 300/817/1582/1739/1791：未发现新的报文级冲突。残留差异均为产品约定（CLI、退出码、`--test`/`--simulation` 默认真、打印 CSV）或标准内部已注明取舍（Table 73 `r` vs 7-2 可写）。付费 7-2/8-1/9-2 全文不在仓库；以本地 2007C5 + 样章 + tissue + 互通现象为准，本轮视为收敛。
