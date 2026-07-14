#import "@preview/touying:0.7.4": *
#import themes.university: *

#let fonts = (
  mono: "New Computer Modern Mono",
  serif: "New Computer Modern",
  sans: "New Computer Modern Sans",
  math: "New Computer Modern Math",
  cjk-serif: "Source Han Serif SC",
  cjk-sans: "Sarasa UI SC",
)

#let academic-table(columns: (), ..cells) = table(
  columns: columns,
  stroke: (x: none, y: 0.6pt + gray),
  inset: (x: 0.6em, y: 0.45em),
  ..cells,
)

#show: university-theme.with(
  aspect-ratio: "16-9",
  align: horizon,
  progress-bar: true,
  config-common(
    breakable: false,
    detect-overflow: true,
    receive-body-for-new-section-slide-fn: false,
  ),
  config-info(
    title: [Hull],
    short-title: [Hull],
    subtitle: [基于 Nix 与 WASM 的算法竞赛本地造题系统],
    author: [Aberter0x3F],
    date: datetime(year: 2026, month: 7, day: 12),
    institution: [TUIQUN '26],
  ),
)

#set text(font: (fonts.sans, fonts.cjk-sans), lang: "zh")
#show raw: set text(
  font: (fonts.mono, fonts.sans, fonts.cjk-sans),
  // Typst's default raw style scales text to 0.8em; 1.25em restores the visible size to 1em.
  size: 1.25em,
)

#show raw.where(block: true): set text(size: 0.8em)

#show raw.where(block: true): set par(leading: 0.4em)

#show math.equation: set text(font: (fonts.math, fonts.serif, fonts.cjk-serif))

#show link: underline

#title-slide()

== Copyright <touying:hidden>

本文档内容采用

#strong[
  #set align(center)
  #[#set text(size: 1.5em); 知识共享 署名--相同方式共享 4.0 协议]\
  CC BY-SA 4.0
]

#image("assets/cc-by-sa.png", width: 20%)

== Outline <touying:hidden>

#components.adaptive-columns(outline(title: none, indent: 1em, depth: 1))

= 引言

== 从 WASM Judge 到 Hull

=== 去年的问题

根据 fstqwq 的意见, 本项目如果要普及需要一个与它非常适配的情景.

#align(center)[*基于 Nix 的本地造题系统?*]

---

WASM Judge 已经实现了评测尺度稳定, 允许安全并行. 但作为造题系统, 仍需要解决 *题目信息声明*, *工具链依赖管理*, *自动化打包流程*.

我们需要一个能做到上述几点的设计.

== Why to choose Nix?

或许我们可以从 Nix 是什么来回答.

---

Nix 是版本管理工具.

```nix
inputs = {
  nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
  cplib = ...;
  hull = ... ;
};

outputs = {self, nixpkgs, hull, cplib}: let ... in {
  hullProblems = {
    major = hullLib.evalProblem ./problem/major { };
    stone = hullLib.evalProblem ./problem/stone { };
    count = hullLib.evalProblem ./problem/count { };
  };
  hullContests.default = hullLib.evalContest ./contest.nix { };
}
```

`flake.nix` + `flake.lock` 可声明, 锁定工具链.

---

Nix 是函数式编程语言.

```nix
{
  validator = { src = ./validator.cpp; tests = ...; };
  checker = { src = ./checker.cpp; tests = ...; };
  testCases = {
    random-small = {
      generator = "rand";
      arguments = [ "--n-max=100" ];
    };
  };
  traits = { n_le_100 = { descriptions.en = "$N <= 100$."; }; };
  subtasks = [
    { traits = { n_le_100 = true; }; fullScore = 0.4; }
    { fullScore = 0.6; }
  ];
```

---

```nix
  solutions = {
    src = ./solution/std.20.cpp;
    mainCorrectSolution = true;
    subtaskPredictions = {
      "0" = { score, ... }: score == 1.0;
      "1" = { score, ... }: score == 1.0;
    };
  };
  documents = { ... };
  targets = {
    default = hull.problemTarget.common;
    hydro = hull.problemTarget.hydro.batch { statements.en = "statement.en.pdf"; };
  };
}
```

代替各种 `problem.json`, `problem.yaml`, 声明式定义题目.

---

Nix 是脚本语言.

#grid(
  columns: 2,
  [
    ```bash
    $ ls nix/lib/problemTarget
    .
    ├── hydroCustom
    │   ├── checker.c
    │   └── default.nix
    ├── lemonCustom
    │   ├── default.nix
    │   └── watcher.c
    ├── luogu
    │   ├── default.nix
    │   └── wrapper.c
    ├── uoj
    │   ├── default.nix
    │   └── wrapper.c
    ```
  ],
  [
    ```bash
    ├── uojCustom
    │   ├── README.txt
    │   ├── default.nix
    │   ├── judger.c
    │   ├── judger.mk
    │   └── judger.sh.in
    ├── cms.nix
    ├── common.nix
    ├── default.nix
    ├── domjudge.nix
    ├── hydro.nix
    └── lemon.nix
    ```
  ],
)

自定义 judger？外接自定义 target?

---

题目是一份可执行规格.

#align(center)[声明 $arrow.r$ 构建 $arrow.r$ 分析 $arrow.r$ 验证 $arrow.r$ 交付]

We can do them all in one.

== 声明式题目模型

#academic-table(
  columns: (auto, 1fr),
  [*程序*],
  [generator, validator, checker, solutions],
  [*数据*],
  [test cases, traits, subtasks],
  [*验证*],
  [组件测试, 解法预测, 运行断言],
  [*交付*],
  [题面, overview, 单题与比赛 targets],
)

这些部分共同参与同一次求值. validator 输出的 traits 决定测试点所属的 subtask; main correct solution 生成标准输出; 其他解法的实际得分必须符合声明中的预测. 文档和平台包则读取同一份运行分析结果.

---

== 构建流程

#align(center)[#image("assets/generated/architecture.pdf", width: 115%)]

第一次 Nix 求值产生静态 metadata 和待实现的 WASM artifacts. Rust runtime 实现这些 artifacts, 并行运行 validator, checker 和 solutions. 分析结果加入 Nix store 后回注同一组 modules, 由第二次求值检查完整断言并构建 target.

= Features

== 通用

沙盒式评测, 构建, 验证, 打包.

正常造题系统该有的我们都有.

不会还有出题系统运行选手程序没沙盒吧? \@tuack

== Traits

- #sym.crossmark: 声明每个 test case 属于哪些 subtask.
- #sym.checkmark: 声明每个 subtask 要求有什么 "特性", 然后自动匹配.

#grid(
  columns: 2,
  [
    ```nix
    traits = {
      w_eq_1 = { };
      n_le_100 = { };
    };

    subtasks = [
      {
        traits = { w_eq_1 = true; };
        fullScore = 0.2;
      }
      {
        traits = { n_le_100 = true; };
        fullScore = 0.3;
      }
      { fullScore = 0.5; }
    ];
    ```
  ],
  [
    ```cpp
    inline auto traits(const Input &input) -> std::vector<cplib::validator::Trait> {
      return {
        {"w_eq_1", [&input]() {
          for (const auto &e: input.edges) if (e.w != 1) return false;
          return true;
        }},
        {"n_le_100", [&input]() { return input.n <= 100; }},
      }
    }
    ```
  ],
)

---

渲染到题面为:

#[
  #let problem = (
    traits: (w_eq_1: (), n_le_100: ()),
    subtasks: (
      (traits: (w_eq_1: true), full-score: 0.2),
      (traits: (n_le_100: true), full-score: 0.3),
      (traits: (:), full-score: 0.5),
    ),
  );

  #let breakable-text(s) = {
    s.clusters().join(sym.zws)
  }

  #table(
    columns: (0.5fr, 1fr) + (1fr,) * problem.traits.len(),
    align: (
      left + bottom,
      center + bottom,
      ..problem.traits.keys().map(_ => center + bottom),
    ),
    stroke: none,
    table.header(
      repeat: true,
      [*\#*],
      [*Score*],
      ..problem.traits.keys().map(x => breakable-text(x)),
    ),
    table.hline(),
    ..problem
      .subtasks
      .enumerate()
      .map(((id, st)) => {
        (
          ([#id], $#str(st.full-score)$)
            + problem
              .traits
              .keys()
              .map(trait => {
                if not st.traits.keys().contains(trait) {
                  table.cell(fill: yellow.lighten(60%))[?]
                } else if st.traits.at(trait) {
                  table.cell(fill: green.lighten(60%))[#sym.checkmark]
                } else {
                  table.cell(fill: red.lighten(60%))[$times$]
                }
              })
        )
      })
      .flatten(),
  )
]

---

- 防止 subtask 分错 / 分漏, 每个 test case 自动匹配到最严格的 subtask.
- 搬题时无需考虑 subtask 对应关系, 改为自动化分配.
- Hack 直接加入对应 subtask.

== 题面集成


#[
  #set text(size: 0.7em)
  ```cpp
  struct TestCaseInput {
    std::int32_t idx, n, m;
    std::vector<Edge> edges;

    static auto read(cplib::var::Reader &in, std::int32_t tc_idx) -> TestCaseInput {
      std::int32_t n, m;
      std::tie(n, std::ignore, m, std::ignore) = in(cplib::var::i32("n", 1, 2e5), cplib::var::space, cplib::var::i32("m", 1, 2e5), cplib::var::eoln);
      auto edges = in.read(cplib::var::Vec(cplib::var::ExtVar<Edge>("edges", n), m, cplib::var::eoln));
      in.read(cplib::var::eoln);

      if (in.get_trace_level() >= cplib::trace::Level::FULL) {
        in.attach_tag(
          "hull/graph",
          cplib::json::Map{{"name", std::format("graph_{}", tc_idx)},
              {"nodes", std::views::iota(1, n + 1) | std::views::transform([](std::int32_t x) -> std::string { return std::to_string(x); })},
              {"edges", edges | std::views::transform([](const auto &e) -> cplib::json::Map {
                return {{"u", std::to_string(e.u)}, {"v", std::to_string(e.v)}, {"w", std::to_string(e.w)}, {"directed", false}}; })}});
        in.attach_tag("hull/case", tc_idx);
      }

      return {.idx = tc_idx, .n = n, .m = m, .edges = std::move(edges)};
    }
  };
  ```
]

---

// 放两张图并列, mst 题目的 xcpc template 中的 test case & graph vis.

== Custom judger

Hull 原生支持用 judger 自定义评测流程而非预制有限种评测流程. 或者说, 现有的传统题, IO 交互题, 提交答案题在 Hull 中都视作一种特殊的 judger.

Eg: #link("https://uoj.ac/problem/593")[UOJ 新年的军队] 使用两阶段评测:

#align(
  center,
)[执行 1 $arrow.r$ 检查 1 $arrow.r$ 变换输入 $arrow.r$ 验证 2 $arrow.r$ 执行 2 $arrow.r$ 检查 2]

资源使用取两个阶段的最大值, 输出两个文件 `first` 与 `second`.

Hull 不假设 "一次运行, 一次 checker". 题目可以把前一阶段输出转换为后一阶段输入, 再由 validator 检查转换结果. 最终仍由统一 runtime data 表达整个评测过程.

== Custom target

Nix 语言具有极高扩展性.

```nix
targets = {
  default = hull.problemTarget.common;
  hydroCustom = hull.problemTarget.hydroCustom { statements = { en = "statement.en.pdf"; }; };
  lemonCustom = hull.problemTarget.lemonCustom { solutionExtNames = lib.mapAttrs (_: _: "cpp") config.solutions; };
  uojCustom = hull.problemTarget.uojCustom { };
  uojCustomAarch64 = hull.problemTarget.uojCustom {
    targetSystem = "aarch64-linux";
  };
  myCustomTarget = import ./myCustomTarget.nix {}; # 自定义外接 target
};
```

遇到未适配的 OJ 可以编写附加 target 手动适配, 而无需更改 Hull 自身源码.

== 并行执行

=== 分析任务图

运行时分析包含三类独立任务:

- validator tests
- checker tests
- test cases $times$ solutions

这些任务在有界 Rayon 线程池中并行执行.

Hull 还在每个测试点内并行评测多份解法. Tick 独立于共享墙钟和任务抢占, 因而并行度变化保持时限语义稳定. Custom target 的 scheduler 复用同一并行模型.

---

=== Kunpeng 实测

#academic-table(
  columns: (1fr, 2fr),
  [*硬件*],
  [双路 Kunpeng 920, 128 cores],
  [*比赛*],
  [6 题],
  [*评测规模*],
  [test case $times$ solution > 3000],
  [*runtime 用时*],
  [< 60 s],
  [*整场打包用时*],
  [< 90 s],
)

该实验来自一场 6 题 2 天真实 NOI 风格模拟赛. load average 长期超过 100, 说明调度器能够有效利用超算节点的多核并行能力. 相较 tuack 达到数十倍效率提升.

---

=== IOI 赛制的 NOIP

假设全国 NOIP 改用 IOI 赛制 (4 题 5 小时, 每人每题最多提交 50 次), 使用 Hull judger 的基于 WASM 的评测方案, 需要多少台节点?

NOIP 2025 全国共 7,546 个正式参赛者, 四题共 90 个 test case. 通过数学建模不同水平的选手的答题策略, 得到提交次数和提交时间分布. 最终结论为: 平均每人提交 10.81 次, 最后 30 分钟的提交占比 19.78%, 假设全程节点正常工作, 若要保证 95% 的提交在 2 分钟内出结果, 则至少需要 332.7 个 Kunpeng 920 核心 (2.6 个 ARM 超算节点) 或 78.5 个 Intel Xeon Platinum 8358 核心 (1.2 个 x86_64 超算节点).

代码参考项目目录下的 `noip-ioi-capacity/` 目录.

= 部署与比较

== 部署支持

=== 评级标准

#academic-table(
  columns: (auto, 1fr),
  [*Platinum*],
  [部署 Hull runtime, 完整保留评测流程与资源语义],
  [*Gold*],
  [转换为平台原生评测, 保留 subtask 语义, 只支持传统, 交互, 提答三种常见题目类型],
  [*Silver*],
  [转换为平台原生评测, subtask 语义受限或无法覆盖三种常见题目类型],
  [*Bronze*],
  [只能表达基础 batch/pass-fail 模型],
)

评级检查 custom judger, 多输出, validator traits, overlapping subtasks, 逐点限制和资源报告.

---

=== Platinum: Runtime 随题部署

#align(center)[题目包 $arrow.r$ Hull scheduler + WASM runners + Nix closure $arrow.r$ OJ]

#academic-table(
  columns: (auto, 1fr),
  [*Hydro*],
  [`proot` 映射私有 Nix store],
  [*UOJ*],
  [`nix-user-chroot` 启动完整闭包],
  [*Lemon*],
  [watcher + special judge + 完整闭包],
)

保留 custom judger, 多输出, validator 产生的 traits, overlapping subtasks, 逐点限制和资源报告. OJ 负责接收提交和展示结果, Hull runtime 继续负责完整评测语义. 使用自定义的支持并行加速的评测流程.

---

#academic-table(
  columns: (auto, auto, 1fr),
  table.header([*等级*], [*Targets*], [*语义边界*]),
  [Gold],
  [UOJ (L), CMS, Luogu],
  [保留 subtask 与部分分],
  [Silver],
  [Hydro (L), Lemon (L)],
  [按平台模型表达分值, 输出和限制],
  [Bronze],
  [DOMjudge],
  [表达 batch 与 pass-fail],
)

(L) 表示 Legacy, 适用于无法取得平台管理员 / custom judger 权限的情况的使用平台原生评测方案的兼容 target 配置.

== 同类项目比较

#align(center)[#image("assets/generated/radar.pdf", width: 75%)]

---

=== Polygon

- 集中式服务.
- 强制在线操作 WebUI / API, 遇到平台宕机则无法工作.
- 私有线性 revision, 不支持 Git, SVN 等常见版本控制系统.
- 单一 package family, 仅支持导出一种部署格式.
- 评测资源使用官方服务器, 以保证各客户端评测稳定.
- *平台管理员位于私有题目信任域.* _Big Mike is watching you!_

---

=== Tuack

- 本地 Python 工具.
- 依赖宿主工具链.
- QA 和执行模型较弱.
- 平台适配严重陈旧, 缺少现代 OJ target 支持.
- 无法保证因各用户主机速度引起的性能 mismatch.
- 仅使用 ulimit 作为资源限制, 直接使用子进程启动评测进程并在外层轮询 real time 和 memory.

---

=== Tuack-NG

- 本地 Rust CLI.
- 依赖宿主工具链.
- 可重放 seed. 具有一定的可复现性.
- *不支持 validator 检验*.
- Target 数目较少.
- Git 协作和扩展结构较强, 但无法保证因各用户主机速度引起的性能 mismatch.
- *无评测沙盒*, 直接使用子进程启动评测进程并在外层轮询 real time 和 memory.

---

=== Local-first 趋势

#align(center)[本地文件 $arrow.r$ Git $arrow.r$ hooks / CI $arrow.r$ Agent $arrow.r$ 发布平台]

集中式 Web GUI 正在失去默认创作入口地位. 平台退回同步, 审批与发布控制面.

题面, 解法, validator, checker 和 generator 放回本地目录. Git 保存权威历史, hooks 和 CI 执行检查, 平台只在需要协作或发布时同步.

= 结论

== 目前的应用

Hull 已被应用于山东三轮省集的命题, 以及 #link("https://uoj.ac/contest/103")[UOJ Long Round 3] 的 A, F 题命题工作.

预计将在今年于 UOJ 举办一场 UOJ WASM Round, 尽请期待!
