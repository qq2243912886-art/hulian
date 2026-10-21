#!/usr/bin/env python3
"""生成「互联」App 图标（1024x1024 PNG），随后用 `pnpm tauri icon assets/app-icon.png` 生成全平台图标。"""
import os
from PIL import Image, ImageDraw, ImageFilter

S = 1024
OUT = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "assets")
os.makedirs(OUT, exist_ok=True)

def lerp(a, b, t):
    return tuple(round(a[i] + (b[i] - a[i]) * t) for i in range(3))

# 背景：圆角渐变（青蓝 → 靛蓝）
bg = Image.new("RGBA", (S, S), (0, 0, 0, 0))
grad = Image.new("RGBA", (S, S))
gd = ImageDraw.Draw(grad)
c1, c2 = (52, 120, 255), (96, 78, 255)
for y in range(S):
    for_x = y / (S - 1)
    gd.line([(0, y), (S, y)], fill=lerp(c1, c2, for_x) + (255,))
# 轻微高光
hl = Image.new("RGBA", (S, S), (0, 0, 0, 0))
hd = ImageDraw.Draw(hl)
hd.ellipse([-S * 0.35, -S * 0.55, S * 0.95, S * 0.35], fill=(255, 255, 255, 34))
grad = Image.alpha_composite(grad, hl)
mask = Image.new("L", (S, S), 0)
ImageDraw.Draw(mask).rounded_rectangle([0, 0, S - 1, S - 1], radius=int(S * 0.225), fill=255)
bg.paste(grad, (0, 0), mask)

d = ImageDraw.Draw(bg)
W = (255, 255, 255, 255)
# 两个互换箭头：左向上、右向下（文件互传）
def arrow_up(cx, top, bottom, w):
    d.rounded_rectangle([cx - w / 2, top + w * 0.9, cx + w / 2, bottom], radius=w / 2, fill=W)
    d.polygon([(cx - w * 1.15, top + w * 1.15), (cx + w * 1.15, top + w * 1.15), (cx, top - w * 0.15)], fill=W)
    # 圆角箭头尖
    d.ellipse([cx - w * 0.32, top - w * 0.05, cx + w * 0.32, top + w * 0.6], fill=W)

def arrow_down(cx, top, bottom, w):
    d.rounded_rectangle([cx - w / 2, top, cx + w / 2, bottom - w * 0.9], radius=w / 2, fill=W)
    d.polygon([(cx - w * 1.15, bottom - w * 1.15), (cx + w * 1.15, bottom - w * 1.15), (cx, bottom + w * 0.15)], fill=W)
    d.ellipse([cx - w * 0.32, bottom - w * 0.6, cx + w * 0.32, bottom + w * 0.05], fill=W)

w = int(S * 0.105)
left_cx, right_cx = int(S * 0.385), int(S * 0.615)
top, bottom = int(S * 0.255), int(S * 0.745)
arrow_up(left_cx, top, bottom - int(S * 0.02), w)
arrow_down(right_cx, top + int(S * 0.02), bottom, w)

# 中间连接弧线（暗示「互联」）
arc = Image.new("RGBA", (S, S), (0, 0, 0, 0))
ad = ImageDraw.Draw(arc)
ad.arc([S * 0.30, S * 0.315, S * 0.70, S * 0.700], start=110, end=250, fill=(255, 255, 255, 0), width=0)
# 用两条短横线连接左右箭头中段
bar_w = int(S * 0.052)
d.rounded_rectangle([S * 0.385, S * 0.49 - bar_w / 2, S * 0.615, S * 0.49 + bar_w / 2], radius=bar_w / 2, fill=(255, 255, 255, 90))
bg = Image.alpha_composite(bg, arc)

path = os.path.join(OUT, "app-icon.png")
bg.save(path)
print("icon ->", path)
