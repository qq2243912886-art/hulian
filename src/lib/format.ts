/** 互联 · 展示格式化工具 */

export function formatBytes(n: number, digits = 1): string {
  if (!Number.isFinite(n) || n <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.min(Math.floor(Math.log(n) / Math.log(1024)), units.length - 1);
  const v = n / 1024 ** i;
  return `${v.toFixed(i === 0 ? 0 : digits)} ${units[i]}`;
}

export function formatSpeed(bps: number): string {
  if (!bps || bps <= 0) return "—";
  return `${formatBytes(bps)}/s`;
}

export function formatDuration(ms: number): string {
  if (!Number.isFinite(ms) || ms <= 0) return "—";
  const s = Math.round(ms / 1000);
  if (s < 60) return `${s} 秒`;
  const m = Math.floor(s / 60);
  const rest = s % 60;
  if (m < 60) return `${m} 分 ${rest} 秒`;
  const h = Math.floor(m / 60);
  return `${h} 小时 ${m % 60} 分`;
}

export function formatTime(iso: string | number): string {
  const d = typeof iso === "number" ? new Date(iso) : new Date(iso);
  if (Number.isNaN(d.getTime())) return "—";
  const pad = (x: number) => String(x).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

export function timeAgo(iso: string | number): string {
  const t = typeof iso === "number" ? iso : new Date(iso).getTime();
  const diff = Date.now() - t;
  if (!Number.isFinite(diff)) return "—";
  if (diff < 60_000) return "刚刚";
  if (diff < 3600_000) return `${Math.floor(diff / 60_000)} 分钟前`;
  if (diff < 86400_000) return `${Math.floor(diff / 3600_000)} 小时前`;
  if (diff < 7 * 86400_000) return `${Math.floor(diff / 86400_000)} 天前`;
  return formatTime(t);
}

/** 根据扩展名给出简洁的类别（用于文件行左侧色块/图标） */
export function fileKind(name: string): { label: string; tone: string } {
  const ext = (name.split(".").pop() ?? "").toLowerCase();
  const map: Record<string, [string, string]> = {
    png: ["图片", "img"],
    jpg: ["图片", "img"],
    jpeg: ["图片", "img"],
    gif: ["图片", "img"],
    webp: ["图片", "img"],
    heic: ["图片", "img"],
    mp4: ["视频", "video"],
    mov: ["视频", "video"],
    mkv: ["视频", "video"],
    m4a: ["音频", "audio"],
    mp3: ["音频", "audio"],
    wav: ["音频", "audio"],
    pdf: ["文档", "doc"],
    doc: ["文档", "doc"],
    docx: ["文档", "doc"],
    xls: ["表格", "sheet"],
    xlsx: ["表格", "sheet"],
    csv: ["表格", "sheet"],
    ppt: ["演示", "slide"],
    pptx: ["演示", "slide"],
    zip: ["压缩包", "zip"],
    rar: ["压缩包", "zip"],
    "7z": ["压缩包", "zip"],
    fig: ["设计稿", "design"],
    sketch: ["设计稿", "design"],
    apk: ["安装包", "app"],
    dmg: ["安装包", "app"],
    exe: ["安装包", "app"],
    txt: ["文本", "doc"],
    md: ["文本", "doc"],
  };
  const hit = map[ext];
  if (hit) return { label: hit[0], tone: hit[1] };
  return { label: ext ? ext.toUpperCase() : "文件", tone: "file" };
}
