# 00 — 分析项目生命周期：方法如何在时间轴上展开

> 这是 synthesis 层的**骨干文件**。`cheatsheet.md` 回答"面对 X 问题选哪个方法"，本文件回答"一个完整分析项目，方法在时间上如何串起来"。**基于 CIA《Tradecraft Primer》(2009) pp. 38-40 的 "Strategies for Using Structured Analytic Techniques" + "Timeline for Using Analytic Techniques"** —— 这是公开文献中最权威的 SAT 时序指南。

---

## 一、CIA 三阶段生命周期（权威骨架）

CIA Tradecraft Primer 把任何分析项目划分为**三个命名阶段**，每个阶段有特定的方法角色。这是本 synthesis 层所有 playbook 的母框架。

```
┌─────────────────────┐    ┌──────────────────────┐    ┌─────────────────────┐
│  Phase 1: 启动      │ →  │  Phase 2: 假设检验    │ →  │  Phase 3: 最终核校  │
│  Starting Out       │    │  Hypothesis Testing   │    │  A Final Check      │
└─────────────────────┘    └──────────────────────┘    └─────────────────────┘
        ↓                            ↓                            ↓
   拓宽视野、挑战默认         系统检验假设、对抗偏见         定稿前自检、补缺
```

### Phase 1 — 启动 (Starting Out)

**目标**：避免"未审先判"——在锁定答案前，确保重要因素不被遗漏、关键假定不被想当然。

> CIA Primer 原文（p.38）：*"At the beginning of an analytic project, analysts are always wise to consider **brainstorming and assumptions checks** to insure that important factors are not being missed or taken for granted. Similarly, **outside-in-thinking** can sometimes put an analytic project into a broader international context."*

**核心方法组合**（启动期的"四大金刚"）：
- **1.1 启动清单** → 12 问自检（动因、用户、替代答案、方法需求等）
- **2.1 结构化头脑风暴** → 强制生成替代解释，防锚定
- **5.1 关键假定检查 ★** → 把暗藏假定摊到桌面
- **5.5 由表及里思考** → 扫外部环境，避免内向视角

**早期信号方法**：
- **6.4 高影响/低概率分析** → 及早识别"黑天鹅"风险
- **3.3 替代前景分析 / 3.4 多种情景生成** → 想象足够多的未来

**贯穿全程的方法**（CIA 特别强调）：
- **4.5 ACH ★★** → *"ACH, in particular, is a good tool to use **throughout** a project to prevent premature closure and to highlight evidence that is most 'discriminating'"* (CIA Primer p.38)
- **3.5 指标法 ★** → 早期定义、持续监测

### Phase 2 — 假设检验 (Hypothesis Testing)

**目标**：随着假设成型，主动挑战"主流分析路线"，防止团体迷思和证实偏见。

> CIA Primer 原文（p.38-39）：*"As an analytic project takes shape, and hypotheses are being formed about the key intelligence question, it can be appropriate to use one or another **contrarian technique** to challenge the conventional analytic line that is being developed."*

**核心方法组合**：
- **4.5 ACH ★★** → 旗舰假设检验工具，证伪式矩阵
- **6.6 红队分析法 ★** → *"If the assessment contains strong judgments about an adversary's behavior, then challenging this view with a **'Red Team' effort** might be a good corrective to too much of a rational actor approach."* (CIA Primer)
- **6.1 事前分析 ★** → 5 分钟低成本质疑，适合阶段性使用
- **4.7 欺骗识别** → 当涉及对手欺骗可能性时

**配套工具**：
- **情报缺口审查** → *"A **review of intelligence gaps** at this juncture can help give the analysts a better degree of confidence"* (CIA Primer)
- **5.1 关键假定检查**（再次）→ 检查新形成的假定

### Phase 3 — 最终核校 (A Final Check)

**目标**：定稿前做"最后一道防线"——确保没有显见的疏漏。

> CIA Primer 原文（p.39）：*"As the assessment is being finalized, it can still be useful to **review key assumptions** as a sanity check... A **brainstorming session** also may be helpful to insure that no plausible hypothesis has been dismissed or left unaddressed. If a firm consensus has formed around an analytic line and has not been seriously questioned in some time, then a **Devil's Advocacy** exercise could be useful."*

**核心方法组合**：
- **5.1 关键假定检查**（第三次！）→ 终极"理性体检"
- **2.1 结构化头脑风暴**（再次）→ 补漏未考虑的假设
- **6.5 魔鬼代言人** → 当团队共识过早形成时
- **3.5 指标法** → 为交付后的持续监测定义可观察信号

---

## 二、方法时序矩阵（基于 CIA Timeline Grid）

下表整合 CIA Primer 的 Timeline Grid + 原书 55 方法的"与其他方法的关系"。**✓ = 强烈推荐在该阶段使用；○ = 可用但非首选；— = 不典型**。

| 方法 | 阶段 1 启动 | 阶段 2 假设检验 | 阶段 3 最终核校 | 备注 |
|---|:---:|:---:|:---:|---|
| **1.1 启动清单** | ✓ | — | ○ | 启动期骨干 |
| **1.2 AIMS / 1.3 用户清单** | ✓ | — | — | 产品构思 |
| **1.4 问题再定义** | ✓ | ○ | — | 厘清焦点 |
| **1.5 大事记表 / 1.6 分类整理** | ✓ | ○ | — | 实证基础 |
| **1.8 矩阵** | ○ | ✓ | ○ | 贯穿性的呈现工具 |
| **1.9 维恩 / 1.10 网络** | ○ | ✓ | — | 关系可视化 |
| **1.11-1.13 思维图/概念图/流程图** | ✓ | ○ | ✓ | 协作可视化 |
| **2.1 结构化头脑风暴 ★** | ✓ | ○ | ✓ | CIA Timeline 强调 |
| **2.3 名义团体法** | ✓ | — | ○ | 权威压制情境 |
| **2.5 交叉影响 / 2.6 形态分析** | ○ | ✓ | — | 系统化穷举 |
| **2.7 经典象限处理™ / 2.8 远景象限™** | ✓ | ○ | — | 寻找"未知的未知" |
| **3.1-3.4 情景系列** | ✓ | ○ | — | 早期想象多未来 |
| **3.5 指标法 ★** | ✓ | ✓ | ✓ | 全程监测 |
| **3.6 指标验证因子™** | ○ | ✓ | ✓ | 优化指标诊断力 |
| **4.1-4.4 假设生成** | ✓ | ✓ | — | 假设穷举 |
| **4.5 ACH ★★** | ○ | ✓✓ | ✓ | **CIA 强调"贯穿全程"** |
| **4.6 论证图示** | — | ✓ | ✓ | ACH 后续深度检验 |
| **4.7 欺骗识别** | ○ | ✓ | ○ | 涉欺骗情境 |
| **5.1 关键假定检查 ★** | ✓ | ✓ | ✓ | **三阶段都用——CIA Primer 三次提及** |
| **5.2 结构化类比** | ○ | ✓ | — | 历史类比 |
| **5.3 角色扮演 / 5.4 红帽** | — | ✓ | ○ | 对手视角 |
| **5.5 由表及里思考** | ✓ | — | — | 早期外部扫描 |
| **6.1 事前分析 ★** | — | ✓ | ✓ | 5 分钟高性价比 |
| **6.2 结构化自我批判** | — | ✓ | ✓ | 系统化质疑 |
| **6.3 若则分析** | — | ✓ | — | 改变变量 |
| **6.4 高影响/低概率** | ✓ | ○ | — | 早期识别黑天鹅 |
| **6.5 魔鬼代言人** | — | ○ | ✓ | CIA Primer 强调"定稿前" |
| **6.6 红队 ★** | — | ✓ | ○ | 长期对抗式挑战 |
| **6.7 德尔菲** | ○ | ✓ | ○ | 多轮专家收敛 |
| **7.1 对抗性协作 / 7.2 结构化辩论** | — | ✓ | ○ | 团队分歧大时 |
| **8.1-8.7 决策支持系列** | — | ○ | ✓ | 选项权衡 |

---

## 三、四范式闭环（来自原书结语 §5.1）

CIA 三阶段是**时间维度**（横向）；原书四范式是**功能维度**（纵向）。两者交叉构成完整项目地图：

```
                  阶段 1 启动        阶段 2 假设检验       阶段 3 最终核校
                  ─────────────     ─────────────────     ──────────────
范式 Ⅰ 实证      │ 大事记表(1.5)    │ 分类整理(1.6)       │
(事实基础)       │ 指标法(3.5)★     │ 诊断推理(4.4)       │
                  │                  │ 结构化类比(5.2)     │
                  ─────────────     ─────────────────     ──────────────
范式 Ⅱ 量化      │                  │ 矩阵法(1.8)         │
(分析骨架)       │                  │ 网络分析(1.10)      │ 决策矩阵(8.2)
                  │                  │ SWOT(8.5)           │ SWOT(8.5)
                  ─────────────     ─────────────────     ──────────────
范式 Ⅲ 认知      │ 经典象限(2.7)    │ ACH(4.5)★★         │ 事前分析(6.1)★
(偏见对抗)       │ 多种情景(3.4)    │ 关键假定检查(5.1)★  │ 魔鬼代言人(6.5)
                  │ 由表及里(5.5)    │ 红队(6.6)★         │ 关键假定检查(5.1)
                  ─────────────     ─────────────────     ──────────────
范式 Ⅳ 协作      │ 启动清单(1.1)    │ 结构化头脑风暴(2.1) │ 结构化头脑风暴(2.1)
(团队整合)       │ AIMS(1.2)        │ 名义团体法(2.3)     │ 对抗性协作(7.1)
                  │ 思维图(1.11)     │ 德尔菲(6.7)         │
```

> **原书 §5.1**：*"一个完整的情报分析项目理想情况下应跨越四个范式：用实证主义方法（大事记表、指标法）**建立事实基础**；用量化方法（矩阵、网络分析）**结构化分解问题**；用认知心理学方法（ACH、关键假定检查、事前分析）**对抗自身偏见**；用协作方法（头脑风暴、可视化）**整合团队智慧**。"*

---

## 四、关键判断法则（Lifecycle Rules of Thumb）

1. **"贯穿全程"的方法只有 4 个**：ACH (4.5)、关键假定检查 (5.1)、指标法 (3.5)、结构化头脑风暴 (2.1)。把这 4 个反复用，比每阶段换新方法更有效。

2. **"诊断力"驱动阶段切换**：当 ACH 矩阵中**高诊断力的证据已经处理完毕**，且关键假定已三次检查，可以进入阶段 3。否则继续阶段 2。

3. **早期投资回报最高**：阶段 1 花 30 分钟用启动清单 + 关键假定检查，能避免阶段 2 的数日返工。CIA Primer 反复强调"start early"。

4. **"反对意见合法化"是阶段 3 的核心**：到了定稿前，团队常已形成共识，异议被压制。事前分析、魔鬼代言人的核心价值是**仪式化质疑**——把"挑刺"包装成"任务角色"，降低社交成本。

5. **方法不是金科玉律**（原书 §5.3）：CIA Primer 和原书都强调，**资深分析人员会根据问题裁剪方法**。"养成结构化思维的习惯比掌握单一方法更宝贵"——核心是培养**跨范式的系统二思维习惯**。

---

## 五、引用与延伸阅读

- **CIA Tradecraft Primer** (2009), pp. 37-40. https://www.cia.gov/resources/csi/static/Tradecraft-Primer-apr09.pdf — **本文件三阶段框架的主要来源**。
- Heuer & Pherson, *Structured Analytic Techniques for Intelligence Analysis*, 2nd ed. (2015), 结语 §5.1 — 四范式闭环。
- Heuer, "How Does ACH Improve Analysis?" https://pherson.org/wp-content/uploads/2013/06/06.-How-Does-ACH-Improve-Analysis_FINAL.pdf — ACH 在 4 步分析过程中的位置。
- ODNI, ICD 203 *Analytic Standards* (2015). https://www.dni.gov/files/documents/ICD/ICD-203.pdf — **重要诚实声明：ICD 203 设定标准（如"analysis of alternatives"），但不规定具体时序。本文件的时序建议来自 CIA Primer，不是 IC 法规。**

---

## 相关文件

- [`01-threat-assessment.md`](01-threat-assessment.md) — 威胁评估/归因 playbook（M&A 尽调、责任归属）
- [`02-early-warning.md`](02-early-warning.md) — 危机预警 playbook（情景 + 指标）
- [`03-high-controversy.md`](03-high-controversy.md) — 高争议团队冲突 playbook
- [`../method-chains.md`](../method-chains.md) — 命名的方法链（ACH hub、广角镜→显微镜等）
- [`../case-studies.md`](../case-studies.md) — 多方法案例走查
- [`../cheatsheet.md`](../cheatsheet.md) §4 — 4 种典型配方（与本文件互补，更紧凑）
