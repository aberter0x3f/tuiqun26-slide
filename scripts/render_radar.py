#!/usr/bin/env python3
import csv
import math
from pathlib import Path

import matplotlib.pyplot as plt
from matplotlib.font_manager import FontProperties


ROOT = Path(__file__).resolve().parent.parent
DATA = ROOT / "data" / "radar.csv"
OUTPUT = ROOT / "assets" / "generated" / "radar.pdf"

LABELS = [
    "复现",
    "自动化",
    "协作",
    "验证",
    "扩展",
    "交付",
    "数据控制",
    "安全性",
    "性能",
]

STYLES = {
    "Polygon": ("#315D91", "-", "o"),
    "tuack": ("#B5652A", (0, (7, 4)), "s"),
    "tuack-ng": ("#6853A6", (0, (4, 3, 1, 3)), "D"),
    "Hull": ("#176B4D", "-", "^"),
}


with DATA.open(newline="", encoding="utf-8") as source:
    rows = list(csv.DictReader(source))

angles = [index * 2 * math.pi / len(LABELS) for index in range(len(LABELS))]
closed_angles = angles + angles[:1]

font = FontProperties(family="Sarasa UI SC", size=10)
legend_font = FontProperties(family="Sarasa UI SC", size=10)

fig, ax = plt.subplots(figsize=(10.5, 4.4), subplot_kw={"projection": "polar"})
fig.patch.set_facecolor("white")
ax.set_facecolor("white")
ax.set_theta_offset(math.pi / 2)
ax.set_theta_direction(-1)
ax.set_ylim(0, 5)
ax.set_yticks([1, 2, 3, 4, 5])
ax.set_yticklabels([])
ax.set_xticks(angles, LABELS, fontproperties=font)
ax.tick_params(axis="x", pad=11, colors="#171717")
ax.grid(color="#C8C8C8", linewidth=0.7)
ax.spines["polar"].set_color("#888888")
ax.spines["polar"].set_linewidth(0.8)

for row in rows:
    system = row["system"]
    values = [int(value) for key, value in row.items() if key != "system"]
    closed_values = values + values[:1]
    color, linestyle, marker = STYLES[system]
    linewidth = 2.6 if system == "Hull" else 1.8
    ax.plot(
        closed_angles,
        closed_values,
        color=color,
        linestyle=linestyle,
        linewidth=linewidth,
        marker=marker,
        markersize=4.5,
        label=system,
    )
    ax.fill(closed_angles, closed_values, color=color, alpha=0.04)

ax.legend(
    loc="center left",
    bbox_to_anchor=(1.16, 0.58),
    frameon=False,
    prop=legend_font,
    handlelength=3.0,
    labelspacing=1.0,
)
fig.text(
    0.79,
    0.22,
    "1 = 相对弱端\n5 = 相对强端",
    color="#555555",
    fontproperties=FontProperties(family=["Sarasa UI SC"], size=8.5),
    linespacing=1.5,
)

OUTPUT.parent.mkdir(parents=True, exist_ok=True)
fig.savefig(OUTPUT, bbox_inches="tight", pad_inches=0.08, facecolor="white")
plt.close(fig)
