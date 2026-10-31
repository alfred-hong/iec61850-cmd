# IEC 61850 官方文件说明

完整的 IEC 61850 标准正文（PDF）由 IEC 出售，不是公开下载。
2026-10-14 的全套包是 [IEC 61850:2026 SER](https://webstore.iec.ch/en/publication/6028)，目录价约 CHF 26,508。
本目录因此**没有**放入标准 PDF。从非官方站点抓取全文属于侵权，这里不这样做。

## 和本项目直接相关的现行核心部分

| 部分 | 现行合并版 | 官方购买页 |
|------|------------|------------|
| 61850-6 SCL | 2009+AMD1:2018+AMD2:2024（Edition 2.2，命名空间 2007C5） | [webstore 61850 搜索](https://webstore.iec.ch/en/search?q=61850) |
| 61850-7-2 ACSI | 2010+AMD1:2020 CSV | https://webstore.iec.ch/en/publication/66525 |
| 61850-7-3 CDC | 2010+AMD1:2020 CSV | webstore 同上搜索 |
| 61850-7-4 LN | 2010+AMD1:2020 CSV | webstore 同上搜索 |
| 61850-8-1 MMS/GOOSE | 2011+AMD1:2020 CSV | https://webstore.iec.ch/en/publication/66585 |
| 61850-9-2 SV | 2011+AMD1:2020 CSV | webstore 同上搜索 |
| 全套 | IEC 61850:2026 SER（2026-10-14） | https://webstore.iec.ch/en/publication/6028 |

买下后把 PDF 放到本目录即可，文件名保持 IEC 原名。

## 已在本地的 Code Component

IEC 允许免费下载 **Code Component**（SCL 的 XSD、数据模型的 NSD light 包），使用仍受 [IEC CCv1](https://www.iec.ch/CCv1) 许可约束。分发页：

https://www.iec.ch/tc57/supportdocuments

2026-10-29 已手动放入并校验通过：

| 文件 | 内容 |
|------|------|
| `iec-free-code-components/IEC_61850-6.SCL.2007C5.full/` | 已解压的 SCL schema V2007C5，full，清单日期 2024-03-12。对应 IEC 61850-6 Edition 2.2。`SCL.xsd` 的 `version="2007C5"`，`targetNamespace` 为 `http://www.iec.ch/61850/2003/SCL`。根元素 `version`、`revision`、`release` 分别固定为 `2007`、`C`、`5`。 |

包内清单写明：若这里的定义与已出版 PDF 不一致，**以本 code component 为准**。版权声明指向 IEC 61850-6:2009/AMD2:2023。

7-3 / 7-4 的 `*.light.zip` 还没放进这个目录。现行标准正文 PDF 仍不在仓库里。

2026-10-29 已从本目录删除两批无参考价值的 PDF：第 1 版制定期的 CD/CDV/FDIS 草案，以及约 2005–2008 年的第 2 版委员会草案。它们不是现行正式出版物。
