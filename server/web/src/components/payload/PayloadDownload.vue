<template>
  <div class="payload-download">
    <div class="file-info">
      <div class="file-details">
        <div class="file-icon">
          <Icon :name="getFileIcon()" size="32" />
        </div>
        <div class="file-meta">
          <h4 class="filename">{{ downloadInfo.filename }}</h4>
          <p class="filesize">{{ formatFileSize(downloadInfo.size) }}</p>
          <p class="file-type">{{ getFileType() }}</p>
        </div>
      </div>
      
      <div class="file-actions">
        <Button 
          variant="primary" 
          icon="download"
          @click="$emit('download')"
        >
          Download Payload
        </Button>
      </div>
    </div>

    <div class="file-info-grid">
      <div class="info-item">
        <span class="info-label">Generated</span>
        <span class="info-value">{{ formatDate(new Date()) }}</span>
      </div>
      
      <div class="info-item">
        <span class="info-label">Status</span>
        <span class="status-badge status-ready">
          <Icon name="check" size="14" />
          Ready for Download
        </span>
      </div>
    </div>

    <div class="download-instructions">
      <h5>Download Instructions</h5>
      <ul>
        <li>The payload is ready for immediate download</li>
        <li>Transfer securely to target environment</li>
        <li>Ensure proper execution permissions are set</li>
        <li>Monitor listener for incoming connections</li>
      </ul>
    </div>
  </div>
</template>

<script setup>
import Button from '../ui/Button.vue'
import Icon from '../ui/Icon.vue'

const props = defineProps({
  downloadInfo: {
    type: Object,
    required: true
  }
})

defineEmits(['download'])

function getFileIcon() {
  const filename = props.downloadInfo.filename?.toLowerCase() || ''
  if (filename.endsWith('.exe') || filename.endsWith('.dll')) {
    return 'code'
  } else if (filename.endsWith('.elf')) {
    return 'terminal'
  } else if (filename.includes('shellcode')) {
    return 'code'
  }
  return 'download'
}

function getFileType() {
  const filename = props.downloadInfo.filename?.toLowerCase() || ''
  if (filename.endsWith('.exe')) {
    return 'Windows Executable'
  } else if (filename.endsWith('.dll')) {
    return 'Dynamic Link Library'
  } else if (filename.endsWith('.elf')) {
    return 'Linux Executable'
  } else if (filename.includes('shellcode')) {
    return 'Shellcode Binary'
  }
  return 'Binary File'
}

function formatFileSize(bytes) {
  if (!bytes) return '0 Bytes'
  const k = 1024
  const sizes = ['Bytes', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}

function formatDate(date) {
  return date.toLocaleString()
}
</script>

<style scoped>
.payload-download {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.file-info {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: var(--space-4);
  padding: var(--space-4);
  background: var(--tertiary-bg);
  border-radius: var(--radius);
  border: 2px solid var(--success-color);
}

.file-details {
  display: flex;
  align-items: center;
  gap: var(--space-3);
}

.file-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 48px;
  height: 48px;
  background: var(--success-color);
  color: white;
  border-radius: var(--radius);
}

.file-meta h4 {
  margin: 0 0 var(--space-1) 0;
  color: var(--text-color);
  font-size: 1.125rem;
  font-family: var(--font-mono);
}

.file-meta p {
  margin: 0;
  font-size: 0.875rem;
  color: var(--text-secondary);
}

.filesize {
  font-weight: 600;
  color: var(--success-color) !important;
}

.file-actions {
  flex-shrink: 0;
}

.file-info-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-4);
}

.info-item {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.info-label {
  font-size: 0.75rem;
  color: var(--text-secondary);
  text-transform: uppercase;
  font-weight: 600;
  letter-spacing: 0.5px;
}

.info-value {
  font-size: 0.875rem;
  color: var(--text-color);
  font-family: var(--font-mono);
}

.status-badge {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  font-size: 0.75rem;
  font-weight: 600;
  padding: 0.25rem 0.5rem;
  border-radius: var(--radius-sm);
  width: fit-content;
}

.status-ready {
  background: rgba(63, 185, 80, 0.15);
  color: var(--success-color);
}

.download-instructions {
  background: var(--secondary-bg);
  border: 1px solid var(--border-color);
  border-radius: var(--radius);
  padding: var(--space-4);
}

.download-instructions h5 {
  margin: 0 0 var(--space-3) 0;
  color: var(--text-color);
  font-size: 0.875rem;
  font-weight: 600;
}

.download-instructions ul {
  margin: 0;
  padding-left: var(--space-4);
  list-style-type: disc;
}

.download-instructions li {
  margin-bottom: var(--space-2);
  font-size: 0.875rem;
  color: var(--text-secondary);
  line-height: 1.5;
}

.download-instructions li:last-child {
  margin-bottom: 0;
}

@media (max-width: 768px) {
  .file-info {
    flex-direction: column;
    align-items: stretch;
  }
  
  .file-info-grid {
    grid-template-columns: 1fr;
  }
}
</style>