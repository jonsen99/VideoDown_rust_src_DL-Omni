<script lang="ts">
  import { fade, scale } from 'svelte/transition';
  import { expoOut } from 'svelte/easing';

  // Svelte 5 Props 定义
  let { 
    show = false, 
    title = "", 
    onclose, 
    children 
  } = $props<{ 
    show: boolean; 
    title?: string; 
    onclose: () => void; 
    children?: import('svelte').Snippet 
  }>();
</script>

{#if show}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div 
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
    transition:fade={{ duration: 150 }}
    onclick={onclose}
  >
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div 
      class="relative w-full max-w-lg bg-zinc-900 border border-zinc-700/50 rounded-xl shadow-2xl overflow-hidden flex flex-col"
      transition:scale={{ duration: 200, start: 0.95, easing: expoOut }}
      onclick={(e) => e.stopPropagation()}
    >
      <div class="flex items-center justify-between px-6 py-4 border-b border-zinc-800/50">
        <h3 class="text-sm font-medium text-zinc-100">{title}</h3>
        <button 
          class="text-zinc-500 hover:text-zinc-300 transition-colors"
          onclick={onclose}
          aria-label="Close"
          title="Close modal"
        >
          <svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M6 18L18 6M6 6l12 12"/></svg>
        </button>
      </div>
      
      <div class="p-6">
        {#if children}
          {@render children()}
        {/if}
      </div>
    </div>
  </div>
{/if}