<script lang="ts">
  import { platform } from "../lib/api";
  import { store } from "../lib/state.svelte";
  import BrandMark from "./BrandMark.svelte";
  import Icon from "./Icon.svelte";

  const canImportCli = platform() !== "android" && platform() !== "ios";

  const STEPS = [
    {
      title: "连接你的阿里云盘",
      desc: "从本机 aliyun CLI 一键导入，或手动填写 endpoint 与访问令牌。",
    },
    {
      title: "手动上传或下载",
      desc: "拖入文件上传；云端「文件互传」目录里的文件，点一下即可下载。",
    },
    {
      title: "换台设备继续",
      desc: "电脑与手机共用同一个云端目录，随取随用，不中转、不留存。",
    },
  ];

  let importing = $state(false);

  async function importAccount(): Promise<void> {
    if (importing) return;
    importing = true;
    try {
      await store.importCliAccount();
    } finally {
      importing = false;
    }
  }
</script>

<div class="welcome">
  <div class="welcome-card">
    <div class="welcome-head">
      <BrandMark size={56} />
      <h1 class="welcome-title">互联</h1>
      <p class="welcome-tagline">在你自己的阿里云盘之间，轻巧地传文件。</p>
      <p class="welcome-sub">手机与电脑互传，不经过任何中转服务器。</p>
    </div>

    <ol class="steps">
      {#each STEPS as step, index (step.title)}
        <li class="step">
          <span class="step-index num">{index + 1}</span>
          <div class="step-body">
            <p class="step-title">{step.title}</p>
            <p class="step-desc">{step.desc}</p>
          </div>
        </li>
      {/each}
    </ol>

    <div class="welcome-actions">
      {#if canImportCli}
        <button
          type="button"
          class="btn btn-primary btn-lg welcome-btn"
          disabled={importing}
          onclick={importAccount}
        >
          {#if importing}
            <span class="spinner"></span>
          {:else}
            <Icon name="terminal" size={17} />
          {/if}
          {importing ? "正在读取本机配置…" : "从本机 aliyun CLI 导入"}
        </button>
      {/if}
      <button
        type="button"
        class="btn btn-lg welcome-btn"
        class:btn-outline={canImportCli}
        class:btn-primary={!canImportCli}
        onclick={() => store.openSettings(true)}
      >
        <Icon name="settings" size={17} />
        手动填写
      </button>
    </div>

    <p class="privacy">
      <Icon name="shield" size={14} />
      <span>文件只在你自己的阿里云盘之间流转 · 无中间服务器 · 无后台同步</span>
    </p>
  </div>

  <p class="welcome-foot">
    互联 · 轻量跨端互传
    {#if store.appInfo}
      <span class="dot-sep">·</span>v{store.appInfo.version}
    {/if}
  </p>
</div>

<style>
  .welcome {
    display: flex;
    flex: 1 1 auto;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 26px;
    min-height: 0;
    padding: 40px 24px;
    overflow-y: auto;
  }

  .welcome-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    width: min(100%, 520px);
  }

  .welcome-head {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    text-align: center;
  }

  .welcome-title {
    margin: 14px 0 2px;
    color: var(--text);
    font-size: 27px;
    font-weight: 600;
    letter-spacing: 0.14em;
    text-indent: 0.14em;
  }

  .welcome-tagline {
    margin: 0;
    color: var(--text-2);
    font-size: 15px;
    letter-spacing: 0.01em;
  }

  .welcome-sub {
    margin: 0;
    color: var(--text-3);
    font-size: 12.5px;
  }

  .steps {
    width: 100%;
    margin: 26px 0 0;
    padding: 2px 0;
    border: 1px solid var(--border);
    border-radius: var(--r-xl);
    background: var(--surface);
    list-style: none;
  }

  .step {
    display: flex;
    gap: 12px;
    padding: 14px 16px;
  }

  .step + .step {
    border-top: 1px solid var(--border-faint);
  }

  .step-index {
    display: grid;
    flex: 0 0 auto;
    place-items: center;
    width: 22px;
    height: 22px;
    margin-top: 1px;
    border-radius: 7px;
    background: var(--primary-soft);
    color: var(--primary);
    font-size: 11px;
    font-weight: 600;
  }

  .step-body {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .step-title {
    margin: 0;
    color: var(--text);
    font-size: 13px;
    font-weight: 500;
  }

  .step-desc {
    margin: 0;
    color: var(--text-3);
    font-size: 12px;
    line-height: 1.55;
  }

  .welcome-actions {
    display: flex;
    gap: 10px;
    width: 100%;
    margin-top: 26px;
  }

  .welcome-btn {
    flex: 1 1 auto;
  }

  .privacy {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    margin: 18px 0 0;
    color: var(--text-3);
    font-size: 11.5px;
    line-height: 1.6;
    text-align: left;
  }

  .privacy :global(svg) {
    flex: 0 0 auto;
    margin-top: 2px;
  }

  .welcome-foot {
    margin: 0;
    color: var(--text-3);
    font-size: 11px;
    letter-spacing: 0.04em;
  }

  .dot-sep {
    margin: 0 4px;
  }

  @media (max-width: 639px) {
    .welcome {
      justify-content: flex-start;
      gap: 20px;
      padding: 30px 20px calc(30px + env(safe-area-inset-bottom));
    }

    .welcome-title {
      font-size: 24px;
    }

    .welcome-actions {
      flex-direction: column;
      gap: 8px;
      margin-top: 22px;
    }

    .welcome-btn {
      flex: 0 0 auto;
      width: 100%;
    }

    .steps {
      margin-top: 20px;
    }
  }
</style>
