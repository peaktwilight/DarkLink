<template>
  <svg 
    :width="size" 
    :height="size" 
    viewBox="0 0 24 24" 
    fill="none" 
    stroke="currentColor" 
    stroke-width="2" 
    stroke-linecap="round" 
    stroke-linejoin="round"
    class="icon"
  >
    <!-- Refresh icon -->
    <g v-if="name === 'refresh'">
      <polyline points="23,4 23,10 17,10"/>
      <polyline points="1,20 1,14 7,14"/>
      <path d="M20.49 9A9 9 0 0 0 5.64 5.64L1 10m22 4l-4.64 4.36A9 9 0 0 1 3.51 15"/>
    </g>
    
    <!-- Dashboard icon -->
    <g v-else-if="name === 'dashboard'">
      <rect x="3" y="3" width="7" height="7"/>
      <rect x="14" y="3" width="7" height="7"/>
      <rect x="14" y="14" width="7" height="7"/>
      <rect x="3" y="14" width="7" height="7"/>
    </g>
    
    <!-- Plus icon -->
    <g v-else-if="name === 'plus'">
      <line x1="12" y1="5" x2="12" y2="19"/>
      <line x1="5" y1="12" x2="19" y2="12"/>
    </g>
    
    <!-- X icon -->
    <g v-else-if="name === 'x'">
      <line x1="18" y1="6" x2="6" y2="18"/>
      <line x1="6" y1="6" x2="18" y2="18"/>
    </g>
    
    <!-- Check icon -->
    <g v-else-if="name === 'check'">
      <polyline points="20,6 9,17 4,12"/>
    </g>
    
    <!-- Upload icon -->
    <g v-else-if="name === 'upload'">
      <path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z"/>
      <polyline points="14,2 14,8 20,8"/>
      <line x1="12" y1="18" x2="12" y2="12"/>
      <polyline points="9,15 12,12 15,15"/>
    </g>
    
    <!-- Terminal icon -->
    <g v-else-if="name === 'terminal'">
      <polyline points="4,17 10,11 4,5"/>
      <line x1="12" y1="19" x2="20" y2="19"/>
    </g>
    
    <!-- Play icon -->
    <g v-else-if="name === 'play'">
      <polygon points="5,3 19,12 5,21"/>
    </g>
    
    <!-- Stop icon -->
    <g v-else-if="name === 'stop'">
      <rect x="6" y="4" width="4" height="16"/>
      <rect x="14" y="4" width="4" height="16"/>
    </g>
    
    <!-- Delete icon -->
    <g v-else-if="name === 'delete'">
      <polyline points="3,6 5,6 21,6"/>
      <path d="m19,6v14a2,2,0,0,1-2,2H7a2,2,0,0,1-2-2V6m3,0V4a2,2,0,0,1,2-2h4a2,2,0,0,1,2,2V6"/>
    </g>
    
    <!-- Network icon -->
    <g v-else-if="name === 'network'">
      <circle cx="12" cy="12" r="2"/>
      <path d="m16.24 7.76-1.96 1.96"/>
      <path d="m7.76 16.24 1.96-1.96"/>
      <path d="m16.24 16.24-1.96-1.96"/>
      <path d="m7.76 7.76 1.96 1.96"/>
      <circle cx="18" cy="6" r="2"/>
      <circle cx="6" cy="18" r="2"/>
      <circle cx="18" cy="18" r="2"/>
      <circle cx="6" cy="6" r="2"/>
    </g>
    
    <!-- Code icon -->
    <g v-else-if="name === 'code'">
      <polyline points="16,18 22,12 16,6"/>
      <polyline points="8,6 2,12 8,18"/>
    </g>
    
    <!-- Download icon -->
    <g v-else-if="name === 'download'">
      <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
      <polyline points="7,10 12,15 17,10"/>
      <line x1="12" y1="15" x2="12" y2="3"/>
    </g>
    
    <!-- Settings icon -->
    <g v-else-if="name === 'settings'">
      <circle cx="12" cy="12" r="3"/>
      <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1 1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/>
    </g>
    
    <!-- User icon -->
    <g v-else-if="name === 'user'">
      <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"/>
      <circle cx="12" cy="7" r="4"/>
    </g>
    
    <!-- Activity icon -->
    <g v-else-if="name === 'activity'">
      <polyline points="22,12 18,12 15,21 9,3 6,12 2,12"/>
    </g>
    
    <!-- Alert triangle icon -->
    <g v-else-if="name === 'alert-triangle'">
      <path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z"/>
      <line x1="12" y1="9" x2="12" y2="13"/>
      <line x1="12" y1="17" x2="12.01" y2="17"/>
    </g>
    
    <!-- Info icon -->
    <g v-else-if="name === 'info'">
      <circle cx="12" cy="12" r="10"/>
      <line x1="12" y1="16" x2="12" y2="12"/>
      <line x1="12" y1="8" x2="12.01" y2="8"/>
    </g>
    
    <!-- File text icon -->
    <g v-else-if="name === 'file-text'">
      <path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z"/>
      <polyline points="14,2 14,8 20,8"/>
      <line x1="16" y1="13" x2="8" y2="13"/>
      <line x1="16" y1="17" x2="8" y2="17"/>
      <polyline points="10,9 9,9 8,9"/>
    </g>
    
    <!-- Archive icon -->
    <g v-else-if="name === 'archive'">
      <polyline points="21,8 21,21 3,21 3,8"/>
      <rect x="1" y="3" width="22" height="5"/>
      <line x1="10" y1="12" x2="14" y2="12"/>
    </g>
    
    <!-- Fallback X icon -->
    <g v-else>
      <line x1="18" y1="6" x2="6" y2="18"/>
      <line x1="6" y1="6" x2="18" y2="18"/>
    </g>
  </svg>
</template>

<script setup>
const props = defineProps({
  name: {
    type: String,
    required: true
  },
  size: {
    type: [String, Number],
    default: 20
  }
})
</script>

<style scoped>
.icon {
  display: inline-block;
  vertical-align: middle;
}
</style>