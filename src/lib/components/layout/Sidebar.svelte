<script lang="ts">
  import { page } from '$app/stores';
  
  // 导航配置
  const navItems = [
    { name: '任务', path: '/', icon: 'M4 6h16M4 10h16M4 14h16M4 18h16' }, // 列表 Icon
    { name: '嗅探', path: '/sniffer', icon: 'M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z' }, // 搜索/嗅探 Icon
    { name: '历史', path: '/history', icon: 'M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z' } // 时钟/历史 Icon
  ];
</script>

<aside class="w-16 h-full flex flex-col items-center py-4 bg-zinc-900 border-r border-zinc-800">
  <div class="mb-8 w-10 h-10 flex items-center justify-center rounded-xl bg-gradient-to-br from-zinc-700 to-zinc-800 shadow-sm border border-zinc-700/50">
    <svg class="w-5 h-5 text-zinc-100" fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"/>
    </svg>
  </div>

  <nav class="flex flex-col space-y-4 w-full px-2">
    {#each navItems as item}
      {@const isActive = $page.url.pathname === item.path}
      <a 
        href={item.path}
        class="group relative flex flex-col items-center justify-center w-full h-12 rounded-lg transition-colors {isActive ? 'bg-zinc-800 text-zinc-100' : 'text-zinc-500 hover:text-zinc-300 hover:bg-zinc-800/50'}"
        title={item.name}
      >
        <svg class="w-5 h-5 mb-1" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d={item.icon}/>
        </svg>
        {#if isActive}
          <div class="absolute left-0 top-1/2 -translate-y-1/2 w-1 h-6 bg-accent-blue rounded-r-full"></div>
        {/if}
      </a>
    {/each}
  </nav>

  <div class="mt-auto w-full px-2">
    <a 
      href="/settings"
      class="flex flex-col items-center justify-center w-full h-12 rounded-lg transition-colors {$page.url.pathname === '/settings' ? 'bg-zinc-800 text-zinc-100' : 'text-zinc-500 hover:text-zinc-300 hover:bg-zinc-800/50'}"
      title="Settings"
      aria-label="Settings"
    >
      <svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/>
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>
      </svg>
    </a>
  </div>
</aside>