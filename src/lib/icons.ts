/**
 * 互联 · 内联 SVG 图标（24×24 网格，stroke 1.5，手绘路径，零图标库依赖）
 * 每个图标由 path / circle / rect 等基础形状组成，由 Icon.svelte 统一渲染。
 */

export type IconShape =
  | { tag: "path"; d: string }
  | { tag: "circle"; cx: number; cy: number; r: number }
  | { tag: "rect"; x: number; y: number; width: number; height: number; rx: number };

function path(...ds: string[]): IconShape[] {
  return ds.map((d) => ({ tag: "path", d }));
}

function circle(cx: number, cy: number, r: number): IconShape[] {
  return [{ tag: "circle", cx, cy, r }];
}

function rect(x: number, y: number, width: number, height: number, rx = 2): IconShape[] {
  return [{ tag: "rect", x, y, width, height, rx }];
}

export const ICONS = {
  upload: path("M12 16V4", "m7 9 5-5 5 5", "M4 15.5V18a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-2.5"),
  download: path("M12 4v12", "m7 11 5 5 5-5", "M4 15.5V18a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-2.5"),
  transfer: path("M4 8h13", "m14 5 3 3-3 3", "M20 16H7", "m10 13-3 3 3 3"),
  cloud: path("M17.5 19H9a7 7 0 1 1 6.71-9h1.79a4.5 4.5 0 1 1 0 9Z"),
  cloudCheck: path(
    "M17.5 19H9a7 7 0 1 1 6.71-9h1.79a4.5 4.5 0 1 1 0 9Z",
    "m8.5 13.5 2 2 4-4"
  ),
  settings: [
    ...path("M3 6h9", "M16 6h5", "M3 12h5", "M12 12h9", "M3 18h11", "M18 18h3"),
    ...circle(14, 6, 2),
    ...circle(10, 12, 2),
    ...circle(16, 18, 2),
  ],
  folder: path(
    "M3 7.5A1.5 1.5 0 0 1 4.5 6H9l2 2h8.5A1.5 1.5 0 0 1 21 9.5v8A1.5 1.5 0 0 1 19.5 19h-15A1.5 1.5 0 0 1 3 17.5v-10Z"
  ),
  folderOpen: path(
    "M3 8.5V6.5A1.5 1.5 0 0 1 4.5 5H9l2 2h7.5A1.5 1.5 0 0 1 20 8.5V10",
    "M3 10h18l-2.3 7.4a1.5 1.5 0 0 1-1.43 1.1H5.73a1.5 1.5 0 0 1-1.43-1.1L3 10Z"
  ),
  file: path(
    "M13.5 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8.5L13.5 3Z",
    "M13.5 3v5.5H19"
  ),
  image: [
    ...rect(3, 4, 18, 16, 3),
    ...circle(8.5, 9.5, 1.5),
    ...path("m4 17 4.5-4.5 3.5 3.5 3-3 5 5"),
  ],
  video: [...rect(3, 5, 18, 14, 3), ...path("m10 9.5 5 2.5-5 2.5v-5Z")],
  audio: [
    ...path("M9 17.5V5.5l10-1.5v12"),
    ...circle(6.75, 17.5, 2.25),
    ...circle(16.75, 15.5, 2.25),
  ],
  doc: path(
    "M13.5 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8.5L13.5 3Z",
    "M13.5 3v5.5H19",
    "M8.5 13h7",
    "M8.5 17h5"
  ),
  sheet: [
    ...rect(3, 4, 18, 16, 2.5),
    ...path("M3 9.5h18", "M3 14.5h18", "M9.5 4v16", "M14.5 4v16"),
  ],
  slide: [...rect(3, 4, 18, 12, 2.5), ...path("M12 16v3", "M8.5 20h7")],
  archive: [
    ...rect(3, 4, 18, 4, 1),
    ...path("M5 8v10.5A1.5 1.5 0 0 0 6.5 20h11a1.5 1.5 0 0 0 1.5-1.5V8", "M10 12h4"),
  ],
  layers: path(
    "m12 3 9 5-9 5-9-5 9-5Z",
    "m3 12 9 5 9-5",
    "m3 16 9 5 9-5"
  ),
  package: path(
    "m12 3 8 4.5v9L12 21l-8-4.5v-9L12 3Z",
    "m4.5 7.5 7.5 4.5 7.5-4.5",
    "M12 12v9"
  ),
  check: path("m5 12.5 4.5 4.5L19 7"),
  x: path("m6 6 12 12", "m18 6-12 12"),
  refresh: path("M20.5 12a8.5 8.5 0 1 1-2.49-6.01", "M20.5 4v5.5H15"),
  trash: path(
    "M4 7h16",
    "M9.5 7V5.5A1.5 1.5 0 0 1 11 4h2a1.5 1.5 0 0 1 1.5 1.5V7",
    "M6.5 7l.7 11.6a1.6 1.6 0 0 0 1.6 1.6h6.4a1.6 1.6 0 0 0 1.6-1.6L17.5 7",
    "M10 11v5.5",
    "M14 11v5.5"
  ),
  chevronRight: path("m9 6 6 6-6 6"),
  chevronDown: path("m6 9 6 6 6-6"),
  arrowLeft: path("M20 12H4", "m10 6-6 6 6 6"),
  external: path(
    "M14 4h6v6",
    "M20 4l-9 9",
    "M18 14v4a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4"
  ),
  eye: [
    ...path("M2.5 12S6 5.5 12 5.5 21.5 12 21.5 12 18 18.5 12 18.5 2.5 12 2.5 12Z"),
    ...circle(12, 12, 3),
  ],
  eyeOff: path(
    "M4 4l16 16",
    "M9.9 5.8A10.5 10.5 0 0 1 12 5.5c6 0 9.5 6.5 9.5 6.5a18.3 18.3 0 0 1-3.2 4.15",
    "M6.2 6.9A18 18 0 0 0 2.5 12S6 18.5 12 18.5a10 10 0 0 0 3.6-.66",
    "M9.9 9.9a3 3 0 0 0 4.2 4.2"
  ),
  shield: path(
    "M12 3.5 19 6v6.2c0 4.2-3 7.4-7 8.8-4-1.4-7-4.6-7-8.8V6l7-2.5Z",
    "m9 11.7 2 2 4-4"
  ),
  sun: [
    ...circle(12, 12, 4),
    ...path(
      "M12 2v2",
      "M12 20v2",
      "m4.93 4.93 1.41 1.41",
      "m17.66 17.66 1.41 1.41",
      "M2 12h2",
      "M20 12h2",
      "m4.93 19.07 1.41-1.41",
      "m17.66 6.34 1.41-1.41"
    ),
  ],
  moon: path("M20 14.5A8.5 8.5 0 0 1 9.5 4 8.5 8.5 0 1 0 20 14.5Z"),
  monitor: [...rect(3, 4, 18, 12, 2), ...path("M9 20h6", "M12 16v4")],
  plus: path("M12 5v14", "M5 12h14"),
  minus: path("M5 12h14"),
  alert: [...circle(12, 12, 9), ...path("M12 7.5v5", "M12 16v.2")],
  info: [...circle(12, 12, 9), ...path("M12 11v5", "M12 8v.2")],
  terminal: path("m5 8 4 4-4 4", "M12 16h7"),
} satisfies Record<string, IconShape[]>;

export type IconName = keyof typeof ICONS;
