# 02 — 危机预警与前瞻监测 Playbook

> 适用问题：**预测可能出现的多种未来，并为每种未来定义可观察的预警信号**。例：地缘冲突预警、市场崩盘、网络安全入侵早期发现、产品发布后竞争反应、自然灾害前的社会动态。
>
> **核心模式**：穷举情景（Quadrant Crunching™）→ 边界约束（Cone of Plausibility）→ 指标定义（Indicators）→ 指标诊断力验证（Indicators Validator™）→ 持续监测。
>
> **来源**：Pherson Associates "Israel: Leveraging Foresight Techniques to Warn Against Surprise Attack" 博文（2023.11）+ 原书第 3 类情景与指标章节 + Diamond Model / Kill Chain 的 cyber 预警实践。

---

## 一、何时用本 Playbook

✅ **典型触发条件**（满足 ≥2 条）：
- 需要预测 6 个月以上的未来
- 单一最可能情景之外有显著不确定性
- 需要可观察的预警信号体系（不只是"会不会发生"的二元判断）
- 对手有意图制造突然性（surprise attack、政变、市场操纵）
- 你需要持续监测而非一次性判断

❌ **不适用**：
- 短期（<1 个月）、可观察变量明确的监测 → 直接用 3.5 指标法即可
- 已发生的突发事件的事后归因 → 用 [`01-threat-assessment.md`](01-threat-assessment.md)
- 高度确定的预测（趋势明确）→ 不需要多情景

---

## 二、5 阶段流程（Quadrant → Cone → Indicators → Validator → Monitor）

> **核心案例参考**：Pherson 2023.11 "Israel" 博文记载，2008 年孟买前瞻工作坊已预测到"海上渗透 + 多目标同时袭击"情景，该情景在 2023.10 哈马斯袭击中再现。博文强调：**Multiple Scenarios Generation + Quadrant Crunching™ + Cone of Plausibility** 应组合使用。

```
┌──────────────┐   ┌──────────────┐   ┌──────────────┐   ┌──────────────┐   ┌──────────────┐
│ Stage 1:     │ → │ Stage 2:     │ → │ Stage 3:     │ → │ Stage 4:     │ → │ Stage 5:     │
│ 维度选择与   │   │ 情景生成     │   │ 边界约束     │   │ 指标定义     │   │ 诊断力验证   │
│ 2x2 矩阵     │   │ 4-12 个      │   │ Cone of      │   │ 每情景 5-10 │   │ Indicators   │
│ (2.7, 3.4)   │   │ 情景         │   │ Plausibility │   │ 个指标       │   │ Validator™   │
│              │   │              │   │ (3.2)        │   │ (3.5)★       │   │ (3.6)        │
└──────────────┘   └──────────────┘   └──────────────┘   └──────────────┘   └──────────────┘
   半天              半天                半天                半天                半天
```

### Stage 1 — 维度选择与 2×2 矩阵

**操作**（基于 2.7 经典象限处理法™ 或 3.4 多种情景生成法）：
1. **明确首要假设**（你直觉认为最可能的未来）。
2. **识别支撑该假设的所有关键假定**（用 5.1 关键假定检查的轻量版）。
3. **从中选出 2 个最关键且最不确定的假定作为正交维度** —— 这是成功的关键。
4. **阐明每个维度的两端**（极端情况）。
5. **画出 2×2 矩阵**，四象限代表四种可能组合。

**维度选择法则**：
- ✅ 好维度：高度不确定 + 高度影响（如"对手内部权力结构稳定性" × "国际制裁强度"）
- ❌ 坏维度：高度确定（如"明年 GDP 会增长"）或低影响（如"明年 1 月是否下雪"）
- ❌ 相关维度：两个维度描述的是同一件事 → 退化

**何时用 2.7 vs 3.4**：
- **2.7 经典象限处理™**：可用数据少、意外突发概率高（反恐、政变）→ 4 个情景
- **3.4 多种情景生成**：多个驱动力都重要、需要更丰富的情景空间 → 多个 2×2 矩阵，4-12 个情景

### Stage 2 — 情景生成与叙事化

**操作**：
1. 为**每个象限**写一段 1-2 页的情景叙事（不是干瘪的描述，而是"2027 年 X 月，Y 事件发生，导致 Z 后果..."的生动叙事）。
2. **重点审视之前未认真考虑的组合** —— 这是新洞见的主要来源（2.7 的核心价值）。
3. 每个情景标注：**概率**（主观评估）+ **影响**（一旦发生后果多重）。
4. **至少包含 1 个"高影响/低概率"情景**（6.4 高影响/低概率分析法的延伸应用）—— 黑天鹅往往就在这里。

**关键产出**：4-12 个**互斥、叙事化、含概率与影响**的情景。

### Stage 3 — 边界约束（Cone of Plausibility）

**为什么需要 Stage 3**：Stages 1-2 的情景可能过宽或过窄。论点合理性 (3.2) 用"上界-中位-下界"框架约束情景的合理性范围。

**操作**：
1. 选定**关键趋势线**（如"对手军费增长""内部民意两极化"）。
2. 对每条趋势，定义：
   - **上界**（最快/最极端变化）：仍合理的最快速度
   - **中位**（最可能）：当前趋势线性外推
   - **下界**（最慢/最保守）：仍合理的最慢速度
3. **删除超出论点合理性边界的情景** —— 这些是"理论上可能但实际不会发生"的。
4. **保留的情景构成"合理性锥形"内的可能未来集合**。

**与 Stage 1-2 的关系**：Stage 1-2 是发散（生成），Stage 3 是收敛（约束）。

### Stage 4 — 指标定义

**这是本 playbook 与 `01-threat-assessment` 的关键区别** —— 预警系统的核心是**可观察的早期信号**。

**操作**（基于 3.5 指标法 ★）：
1. 对每个情景，列出 **5-10 个可观察指标** —— 如果该情景正在发生，应该能看到什么？
2. **指标必须可观察**：
   - ✅ 好指标："对手党报头版措辞变化""边境演习规模""特定商品进口量"
   - ❌ 坏指标："对手内部民意"（不可观察）"对手意图"（不可直接观察）
3. **指标分类**：
   - **早期指标**（6-12 个月前可见）：最宝贵，提供反应时间
   - **中期指标**（1-3 个月前）
   - **晚期指标**（<1 个月）：基本是确认性，反应时间有限
4. **每个指标设定阈值**：什么变化幅度触发什么级别的警报。

**指标法 (3.5) 的历史地位**：原书称之为"情报分析最古老的形式"，至今仍是预警核心。

### Stage 5 — 指标诊断力验证

**为什么必做**：很多指标看起来合理但**实际无诊断力** —— 在多个情景下都会出现，因此无法区分哪个情景正在成为现实。

**操作**（基于 3.6 指标验证因子法™）：
1. 把所有指标 × 所有情景排成矩阵。
2. 对每个单元格评分：该指标在该情景下是否会出现？强度如何？
3. **诊断力 = 区分情景的能力**：
   - 如果一个指标在所有情景下都出现 → 无诊断力（删除或降级）
   - 如果一个指标只在 1 个情景下出现 → 高诊断力（保留为核心指标）
4. **精简指标清单**：从 30-50 个候选指标，筛出 10-15 个高诊断力核心指标。
5. **定义综合判断规则**：哪几个指标同时触发，应激活哪个情景的应对预案？

**输出**：可执行的预警指标体系 + 应对预案激活规则。

### Stage 6 — 持续监测与定期复审

**预警系统不是一次性项目** —— 必须持续运营。

**操作**：
- **每周/每月**：指标扫描，触发警报则激活对应情景应对
- **每季度**：重新评估情景概率（趋势线是否移动？）
- **每年**：完整复审 —— 新情景是否需要加入？旧情景是否已过期？指标是否仍有效？
- **触发事件时**：突发事件后立即复审 —— 这是哪个情景？是否有未预见的情景？

---

## 三、与其他 Playbook 的关系

- **本 playbook 与 `01-threat-assessment` 的根本区别**：归因是"谁干的"（回溯），预警是"会发生什么"（前瞻）。两者常组合使用：预警触发后，转入归因流程判断当前是哪个情景。
- **本 playbook 的核心是"想象足够多的未来"**（CIA Primer Phase 1 强调）—— 与归因 playbook 的"考虑足够多的解释"对称。
- **Crisis Early Warning 配方**：见 [`../cheatsheet.md`](../cheatsheet.md) §4 "配方 B：战略预警监测"。

---

## 四、网络安全/军事的平行应用

> 来源：Diamond Model of Intrusion Analysis（Caltagirone et al., 2013）+ Cyber Kill Chain + MITRE ATT&CK。这三个框架是 CTI（Cyber Threat Intelligence）领域对 SAT 情景/指标方法的工程化实现。

**映射关系**：
| SAT 方法 | CTI 平行概念 | 说明 |
|---|---|---|
| 3.4 多种情景生成 | **Kill Chain 阶段** | Reconnaissance → Weaponization → Delivery → Exploitation → … |
| 3.5 指标法 | **ATT&CK 战术/技术** | 可观察的对手行为模式（TTPs） |
| 4.5 ACH | **Diamond Model 归因** | 事件→线程→组织，多个归因假设竞争 |
| 2.7 经典象限处理™ | **APT 行为预测** | 攻击者下一步可能走哪条路径 |

**Sergio Caltagirone（Diamond Model 作者）原话**："These three [Diamond, Kill Chain, ATT&CK]... are not conflicting, in fact, they are **complementary — you use all three – together**."（Threat Intel Academy）

**实战提示**：网络安全分析师可以把本 playbook 直接应用到威胁狩猎（threat hunting）—— Stage 1-2 生成可能的攻击路径，Stage 3 约束为合理路径，Stage 4-5 定义可观察的 IOC（Indicators of Compromise）和行为指标。

---

## 五、引用

- Pherson Associates, "Israel: Leveraging Foresight Techniques to Warn Against Surprise Attack" (2023.11). https://pherson.org/blog-posts/israel-foresight-technique/
- Caltagirone, Pendergast, Betz, *Diamond Model of Intrusion Analysis* (2013). https://www.activeresponse.org/wp-content/uploads/2013/07/diamond.pdf
- Caltagirone, "Diamond Model, Kill Chain, and ATT&CK" (Threat Intel Academy, 2020). https://www.threatintel.academy/diamond-model-kill-chain-attack/
- CISA, *Best Practices for Mapping to MITRE ATT&CK* (2023). https://www.cisa.gov/sites/default/files/2023-01/Best%20Practices%20for%20MITRE%20ATTCK%20Mapping.pdf
- 原书第 3 类（情景与指标）章节：[`../chapters/ch03-05-indicators.md`](../chapters/ch03-05-indicators.md)、[`../chapters/ch03-06-indicators-validator.md`](../chapters/ch03-06-indicators-validator.md)。
