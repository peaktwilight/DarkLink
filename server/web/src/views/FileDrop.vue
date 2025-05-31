<template>
  <div class="file-drop-page">
    <div class="page-header">
      <h1>File Drop</h1>
      <Button variant="secondary" icon="refresh" @click="loadFiles">
        Refresh
      </Button>
    </div>

    <StatusMessage 
      v-if="statusMessage.visible"
      :type="statusMessage.type"
      :message="statusMessage.text"
      @dismiss="clearStatusMessage"
    />

    <div class="file-drop-layout">
      <!-- Upload Section -->
      <Card class="upload-section">
        <template #header>
          <h2>Upload Files</h2>
        </template>

        <FileUpload 
          :uploading="uploading"
          :upload-progress="uploadProgress"
          @upload="handleFileUpload"
        />
      </Card>

      <!-- Files List Section -->
      <Card class="files-section">
        <template #header>
          <div class="files-header">
            <h3>Uploaded Files</h3>
            <span class="file-count">{{ files.length }} files</span>
          </div>
        </template>

        <FilesList 
          :files="files"
          :loading="filesLoading"
          @download="downloadFile"
          @delete="deleteFile"
        />
      </Card>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { useApi } from '../composables/useApi'
import Card from '../components/ui/Card.vue'
import Button from '../components/ui/Button.vue'
import StatusMessage from '../components/ui/StatusMessage.vue'
import FileUpload from '../components/fileDrop/FileUpload.vue'
import FilesList from '../components/fileDrop/FilesList.vue'

const { apiGet, apiDelete } = useApi()

// Reactive state
const files = ref([])
const uploading = ref(false)
const uploadProgress = ref(0)
const filesLoading = ref(false)
const statusMessage = ref({
  visible: false,
  type: 'info',
  text: ''
})

onMounted(() => {
  loadFiles()
})

async function loadFiles() {
  filesLoading.value = true
  try {
    const response = await apiGet('/api/file_drop/list')
    files.value = response || []
  } catch (error) {
    showStatusMessage(`Failed to load files: ${error.message}`, 'error')
  } finally {
    filesLoading.value = false
  }
}

async function handleFileUpload(uploadFiles) {
  uploading.value = true
  uploadProgress.value = 0
  
  try {
    const formData = new FormData()
    
    // Add all files to FormData
    for (const file of uploadFiles) {
      formData.append('files', file)
    }

    // Use XMLHttpRequest for upload progress tracking
    const xhr = new XMLHttpRequest()
    
    return new Promise((resolve, reject) => {
      xhr.upload.addEventListener('progress', (event) => {
        if (event.lengthComputable) {
          uploadProgress.value = Math.round((event.loaded / event.total) * 100)
        }
      })

      xhr.addEventListener('load', () => {
        if (xhr.status >= 200 && xhr.status < 300) {
          showStatusMessage(`Successfully uploaded ${uploadFiles.length} file(s)`, 'success')
          loadFiles()
          resolve()
        } else {
          reject(new Error(`Upload failed: ${xhr.status}`))
        }
      })

      xhr.addEventListener('error', () => {
        reject(new Error('Upload failed: Network error'))
      })

      xhr.open('POST', '/api/file_drop/upload')
      xhr.send(formData)
    })
  } catch (error) {
    showStatusMessage(`Failed to upload files: ${error.message}`, 'error')
  } finally {
    uploading.value = false
    uploadProgress.value = 0
  }
}

function downloadFile(filename) {
  const downloadUrl = `/api/file_drop/download/${encodeURIComponent(filename)}`
  window.open(downloadUrl, '_blank')
  showStatusMessage(`Download started: ${filename}`, 'success')
}

async function deleteFile(filename) {
  if (!confirm(`Are you sure you want to delete "${filename}"?`)) return
  
  try {
    await apiDelete(`/api/file_drop/delete/${encodeURIComponent(filename)}`)
    showStatusMessage(`File "${filename}" deleted successfully`, 'success')
    await loadFiles()
  } catch (error) {
    showStatusMessage(`Failed to delete file: ${error.message}`, 'error')
  }
}

function showStatusMessage(text, type = 'info') {
  statusMessage.value = {
    visible: true,
    type,
    text
  }
  
  setTimeout(() => {
    clearStatusMessage()
  }, 5000)
}

function clearStatusMessage() {
  statusMessage.value.visible = false
}
</script>

<style scoped>
.file-drop-page {
  display: flex;
  flex-direction: column;
  gap: var(--space-6);
  height: 100%;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.page-header h1 {
  margin: 0;
  color: var(--text-color);
}

.file-drop-layout {
  display: grid;
  grid-template-columns: 1fr 1.5fr;
  gap: var(--space-6);
  flex: 1;
  min-height: 0;
}

.upload-section {
  height: fit-content;
}

.files-section {
  height: 100%;
}

.files-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.files-header h3 {
  margin: 0;
}

.file-count {
  font-size: 0.875rem;
  color: var(--text-secondary);
  background: var(--tertiary-bg);
  padding: 0.25rem 0.75rem;
  border-radius: var(--radius);
}

@media (max-width: 1024px) {
  .file-drop-layout {
    grid-template-columns: 1fr;
  }
}
</style>