<script lang="ts">
  import { fileKind } from "../lib/format";
  import type { IconName } from "../lib/icons";
  import Icon from "./Icon.svelte";

  interface Props {
    name: string;
    size?: number;
  }

  let { name, size = 36 }: Props = $props();

  const TONE_ICON: Record<string, IconName> = {
    img: "image",
    video: "video",
    audio: "audio",
    doc: "doc",
    sheet: "sheet",
    slide: "slide",
    zip: "archive",
    design: "layers",
    app: "package",
    file: "file",
  };

  const kind = $derived(fileKind(name));
  const icon = $derived(TONE_ICON[kind.tone] ?? "file");
</script>

<span
  class="glyph tone-{kind.tone}"
  style="--glyph-size: {size}px"
  title={kind.label}
  aria-hidden="true"
>
  <Icon name={icon} size={Math.round(size * 0.5)} />
</span>

<style>
  .glyph {
    display: inline-grid;
    flex: 0 0 auto;
    place-items: center;
    width: var(--glyph-size);
    height: var(--glyph-size);
    border-radius: calc(var(--glyph-size) * 0.29);
    background: color-mix(in srgb, currentColor 12%, transparent);
    color: var(--tone-file);
  }

  .tone-img {
    color: var(--tone-img);
  }
  .tone-video {
    color: var(--tone-video);
  }
  .tone-audio {
    color: var(--tone-audio);
  }
  .tone-doc {
    color: var(--tone-doc);
  }
  .tone-sheet {
    color: var(--tone-sheet);
  }
  .tone-slide {
    color: var(--tone-slide);
  }
  .tone-zip {
    color: var(--tone-zip);
  }
  .tone-design {
    color: var(--tone-design);
  }
  .tone-app {
    color: var(--tone-app);
  }
  .tone-file {
    color: var(--tone-file);
  }
</style>
