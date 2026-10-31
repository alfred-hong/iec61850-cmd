# iec61850-cmd 服务端

客户端见 [PLAN_client.md](PLAN_client.md)。等那边 Step 3 和 Step 4 都勾完再开始。库、退出码表和 MMS 客户端命令以客户端计划为准，这里不重写。

## Description

做一个纯 Rust 的 IEC 61850 服务端命令行 `iec61850_server`。不调用 libiec61850，也不用 CGO。验收是被 libiec61850 客户端浏览、读值，并完成一次普通报告的总召唤。不声称和标准全文或一致性实验室一致。

必须用 `--scl` 启动。根元素要在 `http://www.iec.ch/61850/2003/SCL` 里，并且至少有一台带逻辑设备的 IED。ICD、CID、SCD 是预期种类；IID、SED 只要满足这一点也接受。SSD 没有 IED，退出码 2。`version` / `revision` / `release` 不是 `2007` / `C` / `5` 时记到 stderr，不因此拒绝整份文件。不认识的工程段落记下节点名并跳过，不拒绝整份文件。JSON 只覆盖已经存在的叶子初值，不能代替工程文件。

第一刀不做：不带 `--scl` 的空服务端、导出 `.cfg`、文件服务、物理量仿真、写、控制、缓冲报告。这些留在 Step 6。S36、S37 不实现。S40–S44 GSSE 不实现、不计数。对时用操作系统时钟。

## Steps

Step 1 空壳已在 [PLAN_client.md](PLAN_client.md)：`crates/iec61850_server` 的 `iec61850_server version` 能跑，`start --scl` 只解析旗标，不监听。

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

### 移植切片与测试

把 `iec61850ctl` 的服务端行为用 Rust 写进本仓库。对照实现是 `iec61850ctl/pkg/stack/server`，协议栈在 `go-mms` 与 `go-iec61850`。不把 Go 逐行翻译过来。本仓库 MMS 客户端栈留着；服务端是它的应答半边。开始写代码仍等 [PLAN_client.md](PLAN_client.md) 的 Step 3 和 Step 4 勾完。每片测试先写、先红，再写实现。上一片没过，不开始下一片。测完关掉自己起的进程。端口用 `--port 0` 或 `10102` 及以上，不占 102。

#### 模块与边界

| 路径 | 职责 | 不碰 |
|------|------|------|
| `crates/iec61850/src/scl/` | 解析 ICD/CID/SCD，产出只读工程模型 | TCP、BER |
| `crates/iec61850/src/server/store.rs` | 叶子值表，键 `LD/LN$FC$path`，与 go-iec61850 `StoreKey` 同形 | 套接字 |
| `crates/iec61850/src/server/dispatch.rs` | MMS 服务请求 → ACSI 动作；只调 path、data、store、report | TCP |
| `crates/iec61850/src/server/report.rs` | URCB/BRCB 状态机；组 InformationReport 的 AccessResult | 原始套接字 |
| `crates/iec61850/src/server/control.rs` | ctlModel 状态机；解码 Oper/SBOw/Cancel；写回 `stVal` | TCP |
| `crates/iec61850/src/mms/server.rs` | 监听、关联、按 invokeID 回 Confirmed-Response / Confirmed-Error / Reject；发不确认的 InformationReport | SCL XML |
| `crates/iec61850_server/src/lib.rs` | 旗标、`--ready-json`、SIGINT/SIGTERM、调库启动 | 协议细节 |
| `crates/iec61850/tests/server_*.rs` | L0/L1 集成；起进程、读 ready、跑客户端、杀进程 | 手工步骤 |
| `crates/iec61850/tests/fixtures/` | 链到或拷贝 `interop.icd`、`test.icd`、`test-values.json`、`test.csv`；另加 `no-getdirectory.icd`、`no-readwrite.icd`、`ssd-no-ied.ssd` | |

服务处理不碰 TCP：`dispatch` 收已经解开的 `ConfirmedServiceRequest`，回 `ConfirmedServiceResponse` 或 `ServiceFailure`。报告走 `InformationReport`，不走确认分发。客户端栈的 `session`、`cotp`、`acse`、asn1 复用；缺的是服务端角色的状态机和 `InformationReport` 发送。

公共接口（名字固定，实现可换）：

```rust
// crates/iec61850/src/scl/mod.rs
pub fn load_scl(path: &Path, ied_name: Option<&str>) -> Result<ServerModel, SclError>;
pub fn apply_values(model: &ServerModel, store: &mut ValueStore, path: &Path) -> Result<Vec<String>, ValuesError>;
// apply_values 返回「跳过的 Ref」列表，由 CLI 打到 stderr

// crates/iec61850/src/server/mod.rs
pub struct ServerConfig {
    pub host: String,           // 默认 "0.0.0.0"
    pub port: u16,              // 0 = 系统选
    pub max_connections: usize, // 默认 5
    pub files_dir: Option<PathBuf>, // Step 6 才用
}
pub struct RunningServer { /* join handle + local_addr + shutdown */ }
pub async fn start(model: ServerModel, store: ValueStore, cfg: ServerConfig) -> Result<RunningServer, ServerError>;
impl RunningServer {
    pub fn local_addr(&self) -> SocketAddr;
    pub async fn shutdown(self);
}

// crates/iec61850_server：ready JSON 固定键
// {"event":"ready","address":"127.0.0.1:xxxxx","ied_name":"...","adapter":"iec61850_server","version":"<CARGO_PKG_VERSION>"}
```

#### 对端与夹具

| 层 | 对端 | 作用 |
|----|------|------|
| L0 | 无套接字 | BER / SCL 单元测试。字节必须是对端已接受过的形状：会话双工 `00 02`；Read 响应访问说明 EXPLICIT `[0]`；Oper 六元结构无 `operTm` |
| L1 | `target/debug/iec61850_client` | 连 `127.0.0.1`。本客户端一关联就拉全模型，所以从切片 5.3 起，监听必须同时应答目录、`GetVariableAccessAttributes`、RCB 读、数据集属性，否则连 `list lds` 都失败 |
| L2 | `/tmp/iec61850ctl` 或 `iec61850ctl` | 同一份 SCL。先作服务端钉本客户端；本服务端起来后作客户端打回来 |
| L3 | libiec61850 客户端二进制 | Step 5 验收。没有该二进制则跳过，Step 5 勾选保持打开 |

夹具（仓库路径，测试用绝对路径打开）：

| 文件 | 用途 |
|------|------|
| `iec61850ctl/e2e/testdata/interop.icd` | IED `InteropIED`，LD `InteropLD`；`dsInterop`；URCB `urcb01`；13 成员 BRCB `brcb01`；`SPCSO1/2/3` 三种 ctlModel；`SetInt1.setVal` |
| `test.icd` | IED `HV_3_AH_04`；LD `HV_3_AH_04CTRL|PROT|EXT|LD0`；合闸 `Q0GAPC10.SPCSO`、分闸 `Q0GAPC11.SPCSO`，ctlModel=`direct-with-normal-security` |
| `test-values.json` | 21 个 `Leaves`，`Type=BOOL`，`Raw=false` |
| `test.csv` | 与上同 21 点，`name,reference,fc` |
| 测试自造 `no-getdirectory.icd` | 从 interop 去掉 `<GetDirectory/>` |
| 测试自造 `no-readwrite.icd` | 去掉 `<ReadWrite/>` |
| 测试自造 `ssd-no-ied.ssd` | 无 IED，退出码 2 |

L1 起服务端的标准外壳（每片复用，测完 SIGTERM）：

```sh
iec61850_server start --scl <icd> [--values <json>] --ied-name <name> \
  --host 127.0.0.1 --port 0 --ready-json
# 读 stdout 第一行 JSON → ADDRESS
iec61850_client --host <host> --port <port> --connect-timeout 5s --timeout 8s <subcommand>
kill -TERM <server-pid>   # 期望退出码 130
```

#### 切片 5.1 SCL，不监听

**文件：** 新建 `crates/iec61850/src/scl/{mod,parse,model,values}.rs`；`Cargo.toml` 加 XML 依赖。`crates/iec61850_server` 仍不监听，但 `start --scl` 在坏文件时改为退出码 2（不再一律 `尚未实现`）。

**行为：** `load_scl` 读出 LD、LN、DO、DA、数据集、报告、`<Services>`。逻辑设备 MMS 域名：有 `ldName` 用它，否则 IED 名接 `inst`。两份现网夹具把**全名**放在 `inst`（`InteropLD`、`HV_3_AH_04CTRL`），解析结果必须是这些字符串，不要再拼一遍 IED 名。LN 名 = `prefix` + `lnClass` + `inst`（`Q0`+`GAPC`+`10` → `Q0GAPC10`）。不认识的元素记名字并跳过。`--values` 只覆盖已有叶子。

**测试（L0）：**

| 用例 | 输入 | 期望 |
|------|------|------|
| interop 设备 | `interop.icd`，IED=`InteropIED` | LD 含 `InteropLD`；LN 含 `LLN0`、`GGIO1`、`MMXU1`、`MMTR1` |
| interop 数据集 | 同上 | `dsInterop` 成员顺序：先 `GGIO1$ST$SPS1$stVal`，再 `LLN0$ST$Mod$stVal`（点分等价亦可，顺序不得反） |
| interop 报告 | 同上 | 非缓冲 `urcb01`；缓冲 `brcb01`；打印名与现客户端一致：`InteropLD/LLN0.RP.urcb01`、`InteropLD/LLN0.BR.brcb01` |
| test 设备 | `test.icd`，IED=`HV_3_AH_04` | LD：`HV_3_AH_04CTRL`、`HV_3_AH_04PROT`、`HV_3_AH_04EXT`、`HV_3_AH_04LD0` |
| test 控制对象 | 同上 | `Q0GAPC10.SPCSO` 与 `Q0GAPC11.SPCSO` 各有 `ST.stVal`（BOOLEAN）、`CF.ctlModel` 初值整数 `1`、`CO.Oper` 含 `ctlVal` |
| values 覆盖 | `test.icd` + `test-values.json` | store 中 21 个键为 `false`；多写一个不存在的 Ref 时返回跳过列表含该 Ref，不增叶子 |
| 坏输入 | 无文件 / `ssd-no-ied.ssd` / 坏 JSON | 错误；CLI 退出码 2 |
| 版本告警 | 非 `2007/C/5` | warnings 非空，仍 `Ok(model)` |

#### 切片 5.2 关联

**文件：** `crates/iec61850/src/mms/server.rs`（可分子模块）；复用 `session`/`cotp`/`acse`。CLI：`start --scl` 开始监听；Initiate 必须成功。目录未实现前，GetNameList 可回「不支持」或空列表，但不得再出现会话 `0x0200` 一类解码错误。

**行为：** Accept Initiate。`negotiatedVersionNumber` = Integer16 `1`。PDU / outstanding / 嵌套按双方取小。带回本端真实支持的 CBB 与 servicesSupportedCalled。请求版本 ≠ 1 → Initiate 错误，不建关联。会话用户需求大端；双工八位组 `00 02`。正常断开 Conclude；异常 Abort。

**测试：**

| 层 | 断言 | 期望 |
|----|------|------|
| L0 | Accept 的 session requirement | 含八位组 `14 02 00 02` |
| L0 | 请求 version=2 | 无关联 |
| L1 | `--ready-json` 起 `interop.icd`，客户端连上后 `action=release` | 关联建立成功（目录可暂失败，但传输层与 MMS Initiate 成功） |
| L1 | `kill -TERM` 服务端 | 退出码 130；端口消失 |
| L1 | 客户端断开 | 服务端清关联，仍监听 |

#### 切片 5.3 目录与读值

**文件：** `dispatch.rs` 实现 `getNameList`、`getVariableAccessAttributes`、`read`；store 初值来自 SCL DAI 与 `--values`。

| ACSI | MMS |
|------|-----|
| GetServerDirectory(LD) | GetNameList objectClass=DOMAIN，vmdSpecific |
| GetLogicalDeviceDirectory / GetLogicalNodeDirectory | GetNameList objectClass=NamedVariable，domainSpecific；标识可含 `$` |
| GetDataDirectory / GetDataDefinition | GetVariableAccessAttributes |
| GetDataValues | Read |

`moreFollows=true` 时下一次必须带上一次最后一个标识作 `continueAfter`。

**测试：**

| 命令 | 期望 |
|------|------|
| `list lds` | 一行 `InteropLD`，码 0 |
| `list lns --ld InteropLD` | 含 `LLN0`、`GGIO1`、`MMXU1`、`MMTR1`，码 0 |
| `list dos --ld InteropLD --ln GGIO1` | 含 `SPS1`、`SPCSO1`、`SetInt1` |
| `list das --ld InteropLD --ln GGIO1 --do SPCSO1` | 含 `ST stVal`（类型短名与现客户端一致，如 `bool`） |
| `get object --object InteropLD/GGIO1.SPS1.stVal --fc ST` | 头 `reference,fc,value,type,quality,time`；`type=BOOLEAN`；`value` 与同机 `iec61850ctl` 一致（多为 `0`） |
| `get object --csv test.csv`（`test.icd`+`test-values.json`） | 21 行 `value=0`、`type=BOOLEAN`，码 0；两行 `Oper.ctlVal` 的 quality/time 为 `-` |
| L0 分页 GetNameList | 两页拼接无重复、无遗漏 |
| `no-getdirectory.icd` 上 `list lds` | 客户端码 6 |
| `no-readwrite.icd` 上读 SPS1 | 客户端码 6 |

#### 切片 5.4 数据集属性与报告控制块读

**文件：** `getNamedVariableListAttributes`；读 `LLN0$RP$urcb01` / `LLN0$BR$brcb01` 返回现客户端 `ReportControlBlock::from_data` 能解的长度。

**BRCB 长度：** 13 = 第 1 版无 `ResvTms`（`interop` 的 `brcb01` 必须用这个）；14 = 有 `ResvTms`；15 = 末尾 `Owner` 客户端丢掉。URCB：11，或 12 丢 `Owner`。长度不对则关联阶段装模型失败，连 `list lds` 都不行。

**测试：**

| 命令 | 期望 |
|------|------|
| `list dss --ld InteropLD --ln LLN0` | 含 `InteropLD/LLN0.dsInterop` |
| `get ds --dataset InteropLD/LLN0.dsInterop` | 两行 SPS1 与 Mod；库侧请求含 `specificationWithResult` TRUE（字节窗口 `80 01 ff` 或 `80 01 01`） |
| `list reports --ld InteropLD --ln LLN0` | 含 `...RP.urcb01` 与 `...BR.brcb01`，码 0 |
| L0 组 13 成员 BRCB | `reservation_time == None` |
| 无 `GetCBValues` 夹具读 RCB | 码 6 |

#### 切片 5.5 URCB 总召唤

**文件：** `report.rs` + 对 `$RP$` 的写。InformationReport：`variableListName` VMD `"RPT"`。AccessResult 顺序见 [PLAN_client.md](PLAN_client.md) Table 64。

**规则：**

- `Resv` 仅未使能时可写；`Owner` = 客户端 IPv4 的 4 八位组（`127.0.0.1` → `7F 00 00 01`），未占用为空。
- 后来的 `Resv=true` 拒绝。`RptEna=true` 后只允许再写 `RptEna` 与 `GI`。
- `gi` 真：写 `GI=true` 发一条总召唤；`gi` 假：拒绝且报告 0 条。
- 仅 URCB：`RptEna` 假→真时 `SqNum` 置 0（u8）。OptFlds 的 buffer-overflow、entryID 恒假。
- 关联结束：清该客户端的 `Resv`/`Owner`，并使能的 URCB 退出使能。

**测试：**

| 步骤 | 期望 |
|------|------|
| L0 组 GI 报告 | 首项 RptID；inclusion 位数 = 数据集成员数；`TimeOfEntry` 若有则恰 6 字节 |
| `Resv=true` 后读 `Owner` | 4 字节，不是 ASCII 点分文本 |
| `Resv` → `RptEna` → `GI`（gi 真） | 收到 1 条报告；成员值 = 当时 store |
| gi 假写 GI | 写失败；报告 0 条 |
| A 占用后 B `Resv=true` | B 拒；A Conclude 后 B 成功，且 A 侧已退出使能 |
| `RptEna=true` 写 `IntgPd` | 拒 |
| L3 libiec61850 browse+read+GI | 有二进制则必须过；无则 skip，Step 5 不勾 |

#### 切片 5.6 进程出口

**文件：** `crates/iec61850_server`。`--ready-json` → **stdout 一行**；「正在监听」→ **stderr**。

| 命令 | 期望 |
|------|------|
| `start --scl interop.icd --ied-name InteropIED --host 127.0.0.1 --port 0 --ready-json` | JSON：`event=ready`，`adapter=iec61850_server`，`ied_name=InteropIED`，`address` 可连；stderr 有监听提示 |
| 用该 address `list lds` | 码 0，`InteropLD` |
| 无 `--scl` / 坏 SCL / 坏 `--values` | 码 2，不监听 |
| 监听中 SIGTERM | 码 130 |

切片 5.1–5.6 全绿且 L3 通过（或环境 skip 并备注）才勾 Step 5。

#### 切片 6.1 写标量

Write → SetDataValues。`FC=CO` 拒，不改 store。无 `ReadWrite` →「不支持」。

```sh
iec61850_client ... set object --object InteropLD/GGIO1.SetInt1.setVal --fc SP --value 7 --type int --verify
# 码 0；stderr 含 action=write 与 action=read；再 --value 0 --verify 写回
iec61850_client ... set object --object InteropLD/GGIO1.SPCSO1.Oper.ctlVal --fc CO --value 1 --type bool
# 客户端码 2 或 6；stVal 不变
```

#### 切片 6.2 直接控制

仅 `ctlModel=1`。写 `LN$CO$DO$Oper`：`ctlVal`、`origin{orCat=3,orIdent空}`、`ctlNum`、`T`、`Test=false`、`Check` 宽 2 全 0；**无** `operTm`。成功则该对象 `stVal := ctlVal`。合闸与分闸两个独立注册。

```sh
iec61850_client ... control operate --object HV_3_AH_04CTRL/Q0GAPC10.SPCSO --value 1 --type bool --yes
# 码 0；stderr: action=operate HV_3_AH_04CTRL/Q0GAPC10.SPCSO 1
# Q0GAPC10.stVal=1，Q0GAPC11.stVal=0
iec61850_client ... control operate --object HV_3_AH_04CTRL/Q0GAPC11.SPCSO.Oper.ctlVal --value 1 --type bool --yes
# 两者均为 1；再都写回 0
# L0：Oper 缺 Check → 拒，stVal 不变
# L0：ctlModel=0 → 码 6，文案含 status-only
```

#### 切片 6.3 选择、增强安全、取消

对象：`GGIO1.SPCSO2`（2）、`SPCSO3`（4）。Select=读 `$SBO`（肯非空 / 否空串仍成功）。除 `$SBO` 外 CO 读拒。增强安全 CommandTermination 与 LastApplError 顺序同 Notes。Select 成功而 Oper 失败 → 同关联 Cancel；直接失败不 Cancel。

| 步骤 | 期望 |
|------|------|
| 未 Select 写 SPCSO2 Oper | 拒；stVal 不变 |
| Select → Oper true | stVal true；再写回 false |
| Select 后 Oper 缺 ctlNum | 拒；可观察到 Cancel |
| SPCSO3 成功 | 收到 `$Oper` 报告 |
| 构造拒绝 | stderr 含 AddCause；码 6 |

#### 切片 6.4 缓冲报告

对象 `LLN0.BR.brcb01`。`EntryID` 8 八位组；使能不置 `SqNum` 0（u16，溢出回 1）；改 RptID/DatSet/BufTm/TrgOps/IntgPd/PurgeBuf 成功则清缓冲；`ResvTms` 0 / 正秒 / `-1`。

测试：使能前后 `SqNum` 不被清；数据变化报告含 8 字节 EntryID；PurgeBuf 后旧条目不再发；`ResvTms=-1` 时第二客户端占用失败。

#### 切片 6.5 数据集写、建、删

Define/DeleteNamedVariableList 与数据集 Write。无 `DynDataSet` / `SetDataSetValue` →「不支持」。

测试：建两成员临时集，`get ds` 顺序一致；Delete 后目录无该名，`dsInterop` 仍在；无 DynDataSet 时 Define → 码 6。

#### 切片 6.6 文件

`--files-dir` 只读、不递归。Delete/Rename 拒。无 `FileHandling` →「不支持」。目录不存在 → 启动码 2。

测试：放 `hello.bin` 内容 `hello`；list/get 一致；delete 后文件仍在。

#### 切片 6.7 其余服务

定值组、取代（FC=SV）、LCB（不打开 LogEna）、单播 SV、TimeActivatedOperate：有对应 `<Services>` 才做最小路径，否则「不支持」。S36/S37 恒「不支持」。

测试：带 SettingGroups 时切换 ActSG 并读回；无元素时码 6；GetGoReference 码 6 且 SPS1 仍可读。

#### Step 6 收口（L2）

本服务端上用 `iec61850ctl` 客户端依次成功：写 `SetInt1`、合闸/分闸直接控制、SBO `SPCSO2`、SBOw `SPCSO3`、一次 BRCB 相关操作。全过才勾 Step 6。GOOSE/SV 以太网发布仍在客户端计划，不进服务端监听。

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
