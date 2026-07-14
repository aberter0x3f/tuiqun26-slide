#!/usr/bin/env python3
from pathlib import Path

from graphviz import Digraph


ROOT = Path(__file__).resolve().parent.parent
OUTPUT = ROOT / "assets" / "generated" / "architecture"

graph = Digraph("hull", format="pdf")
graph.attr(
    rankdir="LR",
    bgcolor="white",
    pad="0.08",
    nodesep="0.32",
    ranksep="0.55",
    splines="ortho",
    fontsize="24",
    fontname="Sarasa UI SC",
)
graph.attr(
    "node",
    shape="box",
    style="rounded",
    color="#244A73",
    fontcolor="#171717",
    fontname="Sarasa UI SC",
    fontsize="24",
    penwidth="1.2",
    margin="0.14,0.09",
)
graph.attr(
    "edge",
    color="#666666",
    fontcolor="#444444",
    fontname="Sarasa UI SC",
    fontsize="16",
    arrowsize="0.7",
)

with graph.subgraph(name="cluster_spec") as cluster:
    cluster.attr(label="题目仓库", color="#AAB7C4", fontname="Sarasa UI SC")
    cluster.node("spec", "problem.nix\n程序 / 数据 / 断言 / targets")
    cluster.node("lock", "flake.lock\n源码 / 编译器 / 依赖")

with graph.subgraph(name="cluster_build") as cluster:
    cluster.attr(label="可复现构建", color="#AAB7C4", fontname="Sarasa UI SC")
    cluster.node("eval1", "Nix 第一次求值\n静态 metadata + WASM artifacts")
    cluster.node("wasm", "WASM 组件\ngenerator / validator / checker / solutions")

with graph.subgraph(name="cluster_runtime") as cluster:
    cluster.attr(label="Hull runtime analysis", color="#AAB7C4", fontname="Sarasa UI SC")
    cluster.node("run", "并行执行\ntestcase x solution")
    cluster.node("runtime", "runtime data\ntraits / verdict / score / resources")

with graph.subgraph(name="cluster_output") as cluster:
    cluster.attr(label="验证与交付", color="#AAB7C4", fontname="Sarasa UI SC")
    cluster.node("eval2", "Nix 第二次求值\n执行 assertions")
    cluster.node("outputs", "最终产物\n数据 / 文档 / overview / 平台包")

graph.edge("lock", "eval1")
graph.edge("spec", "eval1")
graph.edge("eval1", "wasm")
graph.edge("wasm", "run")
graph.edge("run", "runtime")
graph.edge("runtime", "eval2")
graph.edge("spec", "eval2")
graph.edge("eval2", "outputs")

OUTPUT.parent.mkdir(parents=True, exist_ok=True)
graph.render(OUTPUT, cleanup=True)
