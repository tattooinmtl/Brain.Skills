# 01 — 威胁评估与归因 Playbook

> 适用问题：**在多个竞争解释之间判断"谁干的/为什么/是否会做"**。例：APT 归因、M&A 尽调、责任归属、违约判断、欺骗识别。
>
> **核心模式**：假设空间穷举 → 证伪式矩阵检验 → 假定质疑 → 失败预演 → 对手视角。
>
> **来源**：基于 Pherson Associates 公开的"伊拉克铝管案"案例解（multiple SATs 链式应用）+ Heuer ACH 论文 + 原书 ACH/4.7 欺骗识别章节。

---

## 一、何时用本 Playbook

✅ **典型触发条件**（满足 ≥2 条即用）：
- 高风险、不能错（涉及重大投资、追责、对外声明）
- 多种解释并存（敌对行动 vs 意外 vs 第三方介入 vs 欺骗）
- 对手有动机 + 能力 + 机会实施欺骗
- 团队已出现"主流答案"，但你需要确保不是团体迷思
- 证据矛盾、不完整或可能被对手操纵

❌ **不适用**：
- 简单的事实查证（直接调查即可，无需 SAT 流程）
- 单一假设的纯描述性问题
- 时间极度紧张（<2 小时）—— 改用 `02-early-warning` 的轻量版

---

## 二、5 阶段流程（MHG → ACH → KAC → Premortem → Red Hat）

> 来源：Pherson Iraq aluminum-tubes case solution。原文："An **Analysis of Competing Hypotheses** and other matrices could be employed **after the MHG** to array the data... Any assumptions... could be... challenged. A **Key Assumptions Check** would offer another means... **Premortem Analysis and Structured Self-Critique** would have allowed analysts to confront the consequences of a spectacularly wrong conclusion..." (Sagepub UPM assets)

```
┌──────────────┐   ┌──────────────┐   ┌──────────────┐   ┌──────────────┐   ┌──────────────┐
│ Stage 1:     │ → │ Stage 2:     │ → │ Stage 3:     │ → │ Stage 4:     │ → │ Stage 5:     │
│ 假设空间穷举 │   │ 证伪式检验   │   │ 假定质疑     │   │ 失败预演     │   │ 对手视角     │
│ (MHG 4.2)    │   │ (ACH 4.5)★★  │   │ (KAC 5.1)★   │   │ (Premortem   │   │ (Red Hat 5.4│
│              │   │              │   │              │   │  6.1 + SSC   │   │ +Red Team    │
│              │   │              │   │              │   │  6.2)        │   │  6.6)        │
└──────────────┘   └──────────────┘   └──────────────┘   └──────────────┘   └──────────────┘
   30-60 min          1-2 days           30-60 min          30 min            1-2 days
```

### Stage 1 — 假设空间穷举（必做，不可跳过）

**为什么先做 MHG（4.2）而非直接 ACH**：Pherson 反复强调——"Resist the urge to determine that your own (or your first) hypothesis should be the focus of your analysis by using the **Multiple Hypothesis Generation**." (Pherson blog "Don't Jump to Conclusions")。直接列假设会让你落入"首选假设占绝对主导"陷阱。

**操作**（基于 4.2 多种假设生成程序法™）：
1. 召集多元化小组（背景、观点、级别不同）。
2. 每人独立写下 1-3 个假设（沉默头脑风暴，防权威压制）。
3. 收集所有假设，白板呈现。
4. 用"何人/何事/如何/何时/何地/为何"6 维分解首选假设，为每维生成替代项。
5. 排列组合，生成完整假设空间。
6. **关键**：合并/精简为**互斥且尽量穷尽（MECE）**的 4-8 个假设——这是后续 ACH 有效性的前提。
7. **加入"欺骗假设"**：对手正在制造我们看到的迹象（4.7 欺骗识别的"MOM—动机/机会/手段"框架辅助）。

**输出**：一份 MECE 假设清单，含至少 1 个"被忽视的"和 1 个"欺骗"假设。

**常见错误**：
- 只列 2 个假设（"是 X / 不是 X"）—— 不是 MECE，且默认了二元
- 漏掉欺骗假设 —— 在威胁评估中是致命的

### Stage 2 — ACH 证伪式检验（旗舰工具）

> ⚠️ 详细操作见 [`../chapters/ch04-05-ach.md`](../chapters/ch04-05-ach.md)。本节只强调与威胁评估相关的关键点。

**ACH 在威胁评估中的特殊配置**：
1. **证据清单分三类**（用 `../patterns.md` Pattern 8 三重清单）：
   - **显性证据**：观察到的、可验证的
   - **假定**（来自 Stage 3 KAC）：与显性证据**同等地位**纳入矩阵
   - **缺失证据**：若某假设正确则应看到、却未观察到的——**威胁评估中最强的证伪工具**
2. **每格评分 C/I/NA**，但威胁评估中**优先标 Is（强不一致）**——这是诊断力最高的单元格。
3. **不一致评分最低的假设 = 最可能假设**（不是最高分）。
4. **必须做敏感度分析**（ACH 第 7 步）：如果关键证据是欺骗性的或可作不同解读，结论如何变化？

**输出**：完整 ACH 矩阵 + 假设排序 + 高诊断力证据清单 + 监控指标。

### Stage 3 — 关键假定质疑（KAC）

**为什么在 ACH 之后做 KAC**：ACH 矩阵的"假定列"是 Stage 2 的产物之一。KAC (5.1) 系统化地审查它们。

**操作**（30-60 分钟）：
1. 从 ACH 矩阵中提取所有"假定"单元格 → 整理为假定清单。
2. 召集小组 + 1-2 名**局外人**（课题外人员）。
3. 扫描**触发词**："永远""必将""一般而言""显然"——这些词暴露了未质疑的假定。
4. 对每项假定执行**四问批判**：
   - 我为何确信它正确？
   - 何种情况下它可能不正确？
   - 它过去正确现在是否仍正确？
   - 若它无效，对分析影响多大？
5. **三级归类**：基本可靠 / 限定条件下正确 / 关键不确定性（即无法证实也无法证伪）。
6. **关键不确定性 → 若则分析 (6.3)**：想象该假定不正确时，整个 ACH 结论如何变化？
7. 把发现反馈回 Stage 2 的 ACH 矩阵——可能需要重评某些单元格。

**Pherson 经验数据**：约**四分之一的关键假定经审查后被毙或降级**。

### Stage 4 — 失败预演（Premortem + Structured Self-Critique）

**为什么在提交前做**：Stages 1-3 已经形成结论，但团队常陷入"满意答案"陷阱。Premortem 强制从未来回头看失败。

**Premortem 6.1 操作**（5-30 分钟）：
1. 主持人宣布："假设一年后我们的归因结论被证明完全错了，最可能的失败原因是什么？"
2. 每人独立写 3-5 个失败假说（**必须具体**，"领导决策失误"是垃圾；"X 副总主导的方案因 Y 信息缺失而误判"才是）。
3. 名义团体法轮流陈述。
4. 讨论每条：是否成立？如何防范？是否需要修改 ACH 结论？

**Structured Self-Critique 6.2 操作**（30-60 分钟，Premortem 的进阶版）：
- 全员"戴上黑帽"，回答 10 个标准问题（来自 6.2 操作步骤）：
  - 是否找到并检验了替代假设？
  - 是否征求了外单位意见？
  - 关键证据的诊断力是否被高估？
  - 异常证据是否被忽视？
  - 信息空白是否被识别？
  - …

**输出**：失败假说清单 + 修改后的 ACH 结论 + 早期预警指标。

### Stage 5 — 对手视角（Red Hat / Red Team）

**何时必须做**：当归因涉及**有意图的对手**（国家行为体、APT 组织、企业竞争对手）—— 对手会思考你的思考。

**Red Hat Analysis 5.4**（短期、轻量）：
- "如果我是对手的决策者，我会怎么想？我的成本-收益是什么？我是否会采取与我所归因的行为一致的行动？"
- 警惕**镜像思维**——假定对手像自己。

**Red Team Analysis 6.6**（长期、重量）：
- 指定 1-3 人扮演对手团队，**完整地重新分析**问题，挑战所有阶段 1-4 的结论。
- CIA Primer：*"challenging this view with a **'Red Team' effort** might be a good corrective to too much of a rational actor approach"*。

**德尔菲法 6.7（可选）**：当需要外部专家独立验证时，1-2 周的多轮收敛。

---

## 三、典型时间预算

| 投资级别 | 总时长 | 推荐流程 |
|---|---|---|
| **轻量**（中等风险） | 2-4 小时 | Stage 1（30min）+ Stage 2 简化 ACH（2h）+ Stage 4 Premortem（5min） |
| **标准**（高风险） | 1-2 天 | 完整 5 阶段，但 Stage 5 用 Red Hat 而非 Red Team |
| **重量**（极高压，如军事/并购） | 1-2 周 | 完整 5 阶段 + Stage 5 用 Red Team + Stage 6（德尔菲） |

---

## 四、与其他 Playbook 的关系

- **本 playbook 是 4.5 ACH 章节的"应用外壳"** —— ACH 章节教你**怎么做矩阵**，本 playbook 教你**把矩阵嵌入完整判断流程**。
- **本 playbook = `../cheatsheet.md` §4 "配方 D：极高压、不能错"的展开版**。
- 若问题主要是**未来预测**（不是归因），用 [`02-early-warning.md`](02-early-warning.md)。
- 若主要难点是**团队分歧**，用 [`03-high-controversy.md`](03-high-controversy.md)。

---

## 五、诚实声明（来自学术文献）

⚠️ **ACH 有效性存在争议**。Wilcox & Mandel (2023) 在 6 个实验中发现 ACH "little to no overall benefit... and may even harm it"（*Journal of Intelligence*）。但 Borg (Brunel PhD thesis) 发现**组合、分层、迭代使用 SATs**（即本 playbook 的做法）显著提高分析严谨性和不确定性处理能力。

**建议措辞**（用于报告）："本归因采用 CIA 标准的 MHG → ACH → KAC → Premortem 链式流程，置信度评估参考 ICD 203 的'analysis of alternatives'标准。鉴于 ACH 单独使用的有效性存在学术争议，本结论已通过多方法交叉验证。"

---

## 六、引用

- Pherson Associates, "Don't Jump to Conclusions – Take the Right Steps!" https://pherson.org/blog-posts/jump-conclusions-steps/
- Iraq aluminum-tubes 案例解 https://uk.sagepub.com/sites/default/files/upm-assets/114762_book_item_114762.pdf
- Heuer, "How Does ACH Improve Analysis?" https://pherson.org/wp-content/uploads/2013/06/06.-How-Does-ACH-Improve-Analysis_FINAL.pdf
- Wilcox & Mandel (2023), "Critical Review of ACH." https://doi.org/10.31234/osf.io/an32t
- Borg, "Effects of Using SATs in Estimative Intelligence" (Brunel PhD thesis).
