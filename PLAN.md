# iec61850-cmd

需求已拆成两份，本文件只作入口。勾选和改决定时改对应那一份，不要把内容写回这里。

| 文件 | 内容 |
|------|------|
| [PLAN_client.md](PLAN_client.md) | 库和 `iec61850_client`。Step 1 到 Step 4：MMS 命令、GOOSE、采样值 |
| [PLAN_server.md](PLAN_server.md) | `iec61850_server`。等客户端 Step 3 和 Step 4 勾完再开始。Step 5 和 Step 6。文内「移植切片与测试」把模块、接口、命令和期望输出写细 |

两份都按 Description / Steps / Notes 写。步骤号不重排，服务表里的「何时」仍指向原来的 Step。
