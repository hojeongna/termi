<script lang="ts">
  let { name, size = 16, class: className = '' }: {
    name: string;
    size?: number;
    class?: string;
  } = $props();

  const icons: Record<string, { paths: string; fill?: boolean }> = {
    edit: {
      paths: '<path d="M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z"/><path d="m15 5 4 4"/>'
    },
    bell: {
      paths: '<path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9"/><path d="M10.3 21a1.94 1.94 0 0 0 3.4 0"/>'
    },
    'bell-off': {
      paths: '<path d="M8.7 3A6 6 0 0 1 18 8c0 4.2 1.2 6.5 2 8"/><path d="M3.3 3.3 21 21"/><path d="M10.3 21a1.94 1.94 0 0 0 3.4 0"/><path d="M4 8c0-.5.1-1 .3-1.4"/><path d="M3 17h11"/>'
    },
    close: {
      paths: '<path d="M18 6 6 18"/><path d="m6 6 12 12"/>'
    },
    play: {
      paths: '<polygon points="6 3 20 12 6 21 6 3"/>',
      fill: true
    },
    trash: {
      paths: '<path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/><line x1="10" x2="10" y1="11" y2="17"/><line x1="14" x2="14" y1="11" y2="17"/>'
    },
    plus: {
      paths: '<path d="M5 12h14"/><path d="M12 5v14"/>'
    },
    settings: {
      paths: '<circle cx="12" cy="12" r="3"/><path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42"/>'
    },
    folder: {
      paths: '<path d="m6 14 1.5-2.9A2 2 0 0 1 9.24 10H20a2 2 0 0 1 1.94 2.5l-1.54 6a2 2 0 0 1-1.95 1.5H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.69.9l.81 1.2a2 2 0 0 0 1.67.9H18a2 2 0 0 1 2 2v2"/>'
    },
    terminal: {
      paths: '<path d="m7 11 2-2-2-2"/><path d="M11 13h4"/><rect width="18" height="18" x="3" y="3" rx="2" ry="2"/>'
    },
    link: {
      paths: '<path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/>'
    },
    pin: {
      paths: '<path d="M12 17v5"/><path d="M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V7a1 1 0 0 1 1-1 2 2 0 0 0 0-4H8a2 2 0 0 0 0 4 1 1 0 0 1 1 1z"/>'
    }
  };

  const icon = $derived(icons[name]);
</script>

{#if icon}
  <svg
    class="icon {className}"
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill={icon.fill ? 'currentColor' : 'none'}
    stroke={icon.fill ? 'none' : 'currentColor'}
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
  >
    {@html icon.paths}
  </svg>
{/if}

<style>
  .icon {
    display: inline-block;
    vertical-align: middle;
    flex-shrink: 0;
  }
</style>
