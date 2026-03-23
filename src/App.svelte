<script lang="ts">
  import Sidebar from '$lib/components/Sidebar.svelte';
  import TabBar from '$lib/components/TabBar.svelte';
  import TerminalList from '$lib/components/TerminalList.svelte';
  import SettingsPanel from '$lib/components/SettingsPanel.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { terminalsStore, initTerminalEvents } from '$lib/stores/terminals.svelte';
  import { loadSettings, settingsStore } from '$lib/stores/settings.svelte';
  import { initializeTheme } from '$lib/stores/theme.svelte';
  import { i18n, setLocale } from '$lib/i18n/index.svelte';
  import type { Locale } from '$lib/i18n/types';
  import { MIN_SIDEBAR_WIDTH_PX, MAX_SIDEBAR_WIDTH_PX, DEFAULT_SIDEBAR_WIDTH_PX, DEFAULT_DARK_THEME_ID } from '$lib/constants';

  function isLocale(v: string): v is Locale {
    return v === 'en' || v === 'ko';
  }

  const t = $derived(i18n.t);
  let showSettings = $state(false);
  let sidebarWidth = $state(DEFAULT_SIDEBAR_WIDTH_PX);
  let isResizing = $state(false);

  function startResize(e: MouseEvent) {
    e.preventDefault();
    isResizing = true;
    const startX = e.clientX;
    const startWidth = sidebarWidth;

    function onMouseMove(e: MouseEvent) {
      const newWidth = startWidth + (e.clientX - startX);
      sidebarWidth = Math.max(MIN_SIDEBAR_WIDTH_PX, Math.min(MAX_SIDEBAR_WIDTH_PX, newWidth));
    }

    function onMouseUp() {
      isResizing = false;
      document.removeEventListener('mousemove', onMouseMove);
      document.removeEventListener('mouseup', onMouseUp);
    }

    document.addEventListener('mousemove', onMouseMove);
    document.addEventListener('mouseup', onMouseUp);
  }

  $effect(() => {
    const cleanupPromise = initTerminalEvents();
    loadSettings().then(() => {
      const themeId = settingsStore.settings?.theme ?? DEFAULT_DARK_THEME_ID;
      initializeTheme(themeId);
      const lang = settingsStore.settings?.language;
      if (lang && isLocale(lang)) {
        setLocale(lang);
      }
    });
    return () => {
      cleanupPromise.then(cleanup => cleanup?.());
    };
  });
</script>

<div class="app-layout" class:resizing={isResizing} style="--termi-sidebar-width: {sidebarWidth}px">
  <Sidebar onSettingsClick={() => showSettings = !showSettings} />
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="resize-handle" class:dragging={isResizing} onmousedown={startResize}></div>
  <main class="main-content">
    {#if showSettings}
      <SettingsPanel onClose={() => showSettings = false} />
    {:else}
      <TabBar />
      <TerminalList />
      {#if !terminalsStore.hasTerminals}
      <div class="empty-state">
        <Icon name="terminal" size={48} class="empty-icon" />
        <h1>{t.app.title}</h1>
        <p>{t.app.emptyState}</p>
        <ol class="onboarding-steps">
          <li>{t.app.onboarding.step1}</li>
          <li>{t.app.onboarding.step2}</li>
          <li>{t.app.onboarding.step3}</li>
          <li>{t.app.onboarding.step4}</li>
        </ol>
      </div>
      {/if}
    {/if}
  </main>
</div>

<style>
  .app-layout {
    display: flex;
    height: 100%;
    width: 100%;
  }

  .resize-handle {
    width: 4px;
    cursor: col-resize;
    background-color: transparent;
    transition: background-color 0.15s;
    flex-shrink: 0;
  }

  .resize-handle:hover,
  .resize-handle.dragging {
    background-color: var(--termi-accent);
  }

  .app-layout.resizing {
    user-select: none;
    cursor: col-resize;
  }

  .main-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    background-color: var(--termi-bg-primary);
  }

  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    color: var(--termi-text-secondary);
  }

  .empty-state :global(.empty-icon) {
    color: var(--termi-text-secondary);
    margin-bottom: var(--termi-spacing-md);
  }

  .empty-state h1 {
    font-size: var(--termi-font-size-xl, 32px);
    font-weight: 700;
    color: var(--termi-text-primary);
    margin-bottom: var(--termi-spacing-sm, 8px);
  }

  .empty-state p {
    font-size: var(--termi-font-size-md, 15px);
  }

  .onboarding-steps {
    margin-top: var(--termi-spacing-lg, 20px);
    text-align: left;
    color: var(--termi-text-secondary);
    font-size: var(--termi-font-size-sm, 14px);
    padding-left: var(--termi-spacing-lg, 24px);
    line-height: 2;
  }
</style>
