<template>
  <div class="file-upload">
    <div 
      :class="['drop-zone', { 'drag-over': isDragOver, 'uploading': uploading }]"
      @dragover.prevent="handleDragOver"
      @dragleave.prevent="handleDragLeave"
      @drop.prevent="handleDrop"
      @click="triggerFileInput"
    >
      <div class="drop-content">
        <div v-if="uploading" class="upload-state">
          <Icon name="refresh" size="48" class="animate-spin upload-icon" />
          <h3>Uploading files...</h3>
          <div class="progress-bar">
            <div 
              class="progress-fill" 
              :style="{ width: `${uploadProgress}%` }"
            ></div>
          </div>
          <p class="progress-text">{{ uploadProgress }}% complete</p>
        </div>
        
        <div v-else class="drop-state">
          <Icon name="upload" size="48" class="upload-icon" />
          <h3>Drop files here</h3>
          <p>or <span class="browse-link">click to browse</span></p>
          <small class="upload-help">
            Supports multiple files. Maximum 100MB per file.
          </small>
        </div>
      </div>
    </div>

    <input 
      ref="fileInput"
      type="file" 
      multiple 
      style="display: none"
      @change="handleFileSelect"
    >

    <!-- Selected Files Preview -->
    <div v-if="selectedFiles.length && !uploading" class="selected-files">
      <h4>Selected Files ({{ selectedFiles.length }})</h4>
      <div class="files-list">
        <div 
          v-for="(file, index) in selectedFiles" 
          :key="index"
          class="file-item"
        >
          <Icon :name="getFileIcon(file)" size="16" />
          <span class="file-name">{{ file.name }}</span>
          <span class="file-size">{{ formatFileSize(file.size) }}</span>
          <Button 
            variant="danger" 
            size="small"
            icon="x"
            @click="removeFile(index)"
          />
        </div>
      </div>
      
      <div class="upload-actions">
        <Button 
          variant="primary" 
          @click="uploadFiles"
          :disabled="!selectedFiles.length"
        >
          Upload {{ selectedFiles.length }} file{{ selectedFiles.length !== 1 ? 's' : '' }}
        </Button>
        <Button 
          variant="secondary" 
          @click="clearFiles"
        >
          Clear
        </Button>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref } from 'vue'
import Button from '../ui/Button.vue'
import Icon from '../ui/Icon.vue'

const props = defineProps({
  uploading: {
    type: Boolean,
    default: false
  },
  uploadProgress: {
    type: Number,
    default: 0
  }
})

const emit = defineEmits(['upload'])

const fileInput = ref(null)
const selectedFiles = ref([])
const isDragOver = ref(false)

function handleDragOver(event) {
  event.preventDefault()
  isDragOver.value = true
}

function handleDragLeave(event) {
  event.preventDefault()
  isDragOver.value = false
}

function handleDrop(event) {
  event.preventDefault()
  isDragOver.value = false
  
  const files = Array.from(event.dataTransfer.files)
  addFiles(files)
}

function triggerFileInput() {
  if (!props.uploading) {
    fileInput.value?.click()
  }
}

function handleFileSelect(event) {
  const files = Array.from(event.target.files || [])
  addFiles(files)
}

function addFiles(files) {
  // Filter out duplicates and validate files
  const validFiles = files.filter(file => {
    // Check file size (100MB limit)
    if (file.size > 100 * 1024 * 1024) {
      console.warn(`File ${file.name} is too large (max 100MB)`)
      return false
    }
    
    // Check if already selected
    const isDuplicate = selectedFiles.value.some(selected => 
      selected.name === file.name && selected.size === file.size
    )
    
    return !isDuplicate
  })
  
  selectedFiles.value.push(...validFiles)
}

function removeFile(index) {
  selectedFiles.value.splice(index, 1)
}

function clearFiles() {
  selectedFiles.value = []
  if (fileInput.value) {
    fileInput.value.value = ''
  }
}

async function uploadFiles() {
  if (selectedFiles.value.length) {
    await emit('upload', selectedFiles.value)
    clearFiles()
  }
}

function getFileIcon(file) {
  const extension = file.name.split('.').pop()?.toLowerCase()
  
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
  }
  
  return 'download'
}

function formatFileSize(bytes) {
  if (bytes === 0) return '0 Bytes'
  const k = 1024
  const sizes = ['Bytes', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}
</script>

<style scoped>
.file-upload {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.drop-zone {
  border: 2px dashed var(--border-color);
  border-radius: var(--radius-lg);
  padding: var(--space-8);
  text-align: center;
  cursor: pointer;
  transition: all 0.2s ease;
  background: var(--secondary-bg);
  min-height: 200px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.drop-zone:hover {
  border-color: var(--accent-color);
  background: rgba(88, 166, 255, 0.05);
}

.drop-zone.drag-over {
  border-color: var(--accent-color);
  background: rgba(88, 166, 255, 0.1);
  transform: scale(1.02);
}

.drop-zone.uploading {
  cursor: not-allowed;
  border-color: var(--warning-color);
}

.drop-content {
  width: 100%;
}

.upload-state,
.drop-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-3);
}

.upload-icon {
  color: var(--accent-color);
  opacity: 0.8;
}

.drop-state h3 {
  margin: 0;
  color: var(--text-color);
  font-size: 1.25rem;
}

.drop-state p {
  margin: 0;
  color: var(--text-secondary);
}

.browse-link {
  color: var(--accent-color);
  font-weight: 600;
  text-decoration: underline;
}

.upload-help {
  color: var(--text-secondary);
  font-size: 0.875rem;
}

.progress-bar {
  width: 200px;
  height: 8px;
  background: var(--tertiary-bg);
  border-radius: 4px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  background: var(--accent-color);
  transition: width 0.3s ease;
}

.progress-text {
  margin: 0;
  color: var(--text-secondary);
  font-size: 0.875rem;
}

.selected-files {
  border: 1px solid var(--border-color);
  border-radius: var(--radius);
  padding: var(--space-4);
  background: var(--tertiary-bg);
}

.selected-files h4 {
  margin: 0 0 var(--space-3) 0;
  color: var(--text-color);
  font-size: 1rem;
}

.files-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  margin-bottom: var(--space-4);
  max-height: 200px;
  overflow-y: auto;
}

.file-item {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-2) var(--space-3);
  background: var(--secondary-bg);
  border-radius: var(--radius-sm);
  font-size: 0.875rem;
}

.file-name {
  flex: 1;
  color: var(--text-color);
  font-family: var(--font-mono);
  word-break: break-word;
}

.file-size {
  color: var(--text-secondary);
  font-weight: 500;
  white-space: nowrap;
}

.upload-actions {
  display: flex;
  gap: var(--space-3);
  justify-content: flex-end;
}

@media (max-width: 768px) {
  .drop-zone {
    padding: var(--space-6);
    min-height: 150px;
  }
  
  .upload-actions {
    flex-direction: column;
  }
}
</style>