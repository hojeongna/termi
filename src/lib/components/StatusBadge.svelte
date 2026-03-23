<script lang="ts">
    import type { TerminalActivity } from '$lib/types';
    import { i18n } from '$lib/i18n/index.svelte';

    const t = $derived(i18n.t);
    let { status, monitored = true }: { status: TerminalActivity; monitored?: boolean } = $props();
</script>

{#if monitored}
<span
    class="status-badge"
    class:active={status === 'active'}
    class:idle={status === 'idle'}
    title={status === 'active' ? t.terminalList.statusActive : t.terminalList.statusIdle}
></span>
{/if}

<style>
    .status-badge {
        display: inline-block;
        width: var(--termi-status-dot-size, 8px);
        height: var(--termi-status-dot-size, 8px);
        border-radius: var(--termi-radius-full, 50%);
        flex-shrink: 0;
    }

    .active {
        background-color: var(--termi-status-working);
    }

    .idle {
        background-color: var(--termi-status-completed);
    }
</style>
