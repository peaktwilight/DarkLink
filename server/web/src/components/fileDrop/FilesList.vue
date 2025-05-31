<template>
  <div class="files-list">
    <div v-if="loading" class="loading-state">
      <Icon name="refresh" size="32" class="animate-spin" />
      <p>Loading files...</p>
    </div>
    
    <div v-else-if="!files.length" class="empty-state">
      <Icon name="upload" size="48" class="empty-icon" />
      <p>No files uploaded yet</p>
      <p class="text-secondary text-sm">Upload files to see them here</p>
    </div>
    
    <div v-else class="files-table-container">
      <table class="files-table">
        <thead>
          <tr>
            <th>File</th>
            <th>Size</th>
            <th>Modified</th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          <tr 
            v-for="file in files" 
            :key="file.name"
            class="file-row"
          >
            <td class="file-info">
              <Icon :name="getFileIcon(file.name)" size="16" class="file-icon" />
              <span class="file-name">{{ file.name }}</span>
            </td>
            <td class="file-size">{{ formatFileSize(file.size) }}</td>
            <td class="file-modified">{{ formatDate(file.modified) }}</td>
            <td class="file-actions">
              <Button 
                variant="secondary" 
                size="small"
                icon="download"
                @click="$emit('download', file.name)"
              >
                Download
              </Button>
              <Button 
                variant="danger" 
                size="small"
                icon="delete"
                @click="$emit('delete', file.name)"
              >
                Delete
              </Button>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>

<script setup>
import Button from '../ui/Button.vue'
import Icon from '../ui/Icon.vue'

const props = defineProps({
  files: {
    type: Array,
    required: true
  },
  loading: {
    type: Boolean,
    default: false
  }
})

defineEmits(['download', 'delete'])

function getFileIcon(filename) {
  const extension = filename.split('.').pop()?.toLowerCase()
  
  if (['jpg', 'jpeg', 'png', 'gif', 'svg', 'webp'].includes(extension)) {
    return 'image'
  } else if (['mp4', 'avi', 'mov', 'wmv', 'flv'].includes(extension)) {
    return 'video'
  } else if (['mp3', 'wav', 'flac', 'aac'].includes(extension)) {
    return 'music'
  } else if (['pdf'].includes(extension)) {
    return 'file-text'
  } else if (['zip', 'rar', '7z', 'tar', 'gz'].includes(extension)) {
    return 'archive'
  } else if (['exe', 'dll', 'so', 'bin'].includes(extension)) {
    return 'code'
  } else if (['txt', 'md', 'log'].includes(extension)) {
    return 'file-text'
  } else if (['js', 'ts', 'py', 'go', 'rs', 'c', 'cpp', 'java'].includes(extension)) {
    return 'code'
  }
  
  return 'download'
}

function formatFileSize(bytes) {
  if (!bytes || bytes === 0) return '0 Bytes'
  const k = 1024
  const sizes = ['Bytes', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}

function formatDate(dateString) {
  if (!dateString) return 'Unknown'
  try {
    const date = new Date(dateString)
    return date.toLocaleString()
  } catch (error) {
    return 'Invalid date'
  }
}
</script>

<style scoped>
.files-list {
  height: 100%;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.loading-state,
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 300px;
  text-align: center;
}

.empty-icon {
  opacity: 0.5;
  margin-bottom: var(--space-4);
}

.files-table-container {
  flex: 1;
  overflow: auto;
}

.files-table {
  width: 100%;
  border-collapse: collapse;
  background: var(--bg-color);
  border-radius: var(--radius);
  overflow: hidden;
}

.files-table th {
  background: var(--tertiary-bg);
  color: var(--text-color);
  font-weight: 600;
  padding: var(--space-3) var(--space-4);
  text-align: left;
  border-bottom: 1px solid var(--border-color);
  font-size: 0.875rem;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.files-table th:first-child {
  border-top-left-radius: var(--radius);
}

.files-table th:last-child {
  border-top-right-radius: var(--radius);
}

.file-row {
  transition: background-color 0.2s ease;
}

.file-row:hover {
  background: var(--secondary-bg);
}

.file-row td {
  padding: var(--space-3) var(--space-4);
  border-bottom: 1px solid var(--border-color);
  font-size: 0.875rem;
  vertical-align: middle;
}

.file-row:last-child td {
  border-bottom: none;
}

.file-info {
  display: flex;
  align-items: center;
  gap: var(--space-3);
}

.file-icon {
  color: var(--accent-color);
  flex-shrink: 0;
}

.file-name {
  color: var(--text-color);
  font-family: var(--font-mono);
  word-break: break-word;
}

.file-size {
  color: var(--text-secondary);
  font-family: var(--font-mono);
  white-space: nowrap;
}

.file-modified {
  color: var(--text-secondary);
  white-space: nowrap;
}

.file-actions {
  display: flex;
  gap: var(--space-2);
  justify-content: flex-end;
}

/* Mobile responsive */
@media (max-width: 768px) {
  .files-table {
    font-size: 0.8rem;
  }
  
  .files-table th,
  .file-row td {
    padding: var(--space-2) var(--space-3);
  }
  
  .file-actions {
    flex-direction: column;
    gap: var(--space-1);
  }
  
  .file-actions > * {
    width: 100%;
  }
  
  .file-modified {
    display: none;
  }
}

@media (max-width: 480px) {
  .file-size {
    display: none;
  }
  
  .files-table th:nth-child(2),
  .files-table td:nth-child(2) {
    display: none;
  }
}
</style>