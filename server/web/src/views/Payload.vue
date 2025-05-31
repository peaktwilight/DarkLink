<template>
  <div class="payload-page">
    <div class="page-header">
      <h1>Payload Generator</h1>
      <Button variant="secondary" icon="refresh" @click="loadListeners">
        Refresh Listeners
      </Button>
    </div>

    <StatusMessage 
      v-if="statusMessage.visible"
      :type="statusMessage.type"
      :message="statusMessage.text"
      @dismiss="clearStatusMessage"
    />

    <div class="payload-layout">
      <!-- Configuration Panel -->
      <Card class="config-panel">
        <template #header>
          <h2>Agent Configuration</h2>
        </template>

        <PayloadForm 
          :listeners="listeners"
          :loading="generationLoading"
          @generate="generatePayload"
        />
      </Card>

      <!-- Build Output Panel -->
      <div class="output-panel">
        <!-- Download Section -->
        <Card v-if="downloadInfo" class="download-section">
          <template #header>
            <h3>Generated Payload</h3>
          </template>
          
          <PayloadDownload 
            :download-info="downloadInfo"
            @download="downloadPayload"
          />
        </Card>

        <!-- Build Logs -->
        <Card class="build-logs">
          <template #header>
            <div class="logs-header">
              <h3>Build Logs</h3>
              <Button 
                variant="secondary" 
                size="small"
                @click="clearLogs"
              >
                Clear
              </Button>
            </div>
          </template>
          
          <BuildLogs 
            :logs="buildLogs"
            :building="generationLoading"
          />
        </Card>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { useApi } from '../composables/useApi'
import Card from '../components/ui/Card.vue'
import Button from '../components/ui/Button.vue'
import StatusMessage from '../components/ui/StatusMessage.vue'
import PayloadForm from '../components/payload/PayloadForm.vue'
import PayloadDownload from '../components/payload/PayloadDownload.vue'
import BuildLogs from '../components/payload/BuildLogs.vue'

const { apiGet, apiPost } = useApi()

// Reactive state
const listeners = ref([])
const buildLogs = ref([])
const downloadInfo = ref(null)
const generationLoading = ref(false)
const statusMessage = ref({
  visible: false,
  type: 'info',
  text: ''
})

onMounted(() => {
  loadListeners()
})

async function loadListeners() {
  try {
    const response = await apiGet('/api/listeners/list')
    listeners.value = response || []
  } catch (error) {
    showStatusMessage(`Failed to load listeners: ${error.message}`, 'error')
  }
}

async function generatePayload(config) {
  generationLoading.value = true
  downloadInfo.value = null
  buildLogs.value = []
  
  try {
    // Add initial log entry
    addBuildLog('Starting payload generation...', 'info')
    
    const response = await apiPost('/api/payload/generate', config)
    
    if (response.success) {
      downloadInfo.value = {
        filename: response.filename,
        size: response.size,
        downloadUrl: response.downloadUrl
      }
      
      addBuildLog('Payload generated successfully!', 'success')
      showStatusMessage('Payload generated successfully', 'success')
    } else {
      throw new Error(response.error || 'Generation failed')
    }
  } catch (error) {
    addBuildLog(`Generation failed: ${error.message}`, 'error')
    showStatusMessage(`Failed to generate payload: ${error.message}`, 'error')
  } finally {
    generationLoading.value = false
  }
}

function downloadPayload() {
  if (downloadInfo.value?.downloadUrl) {
    window.open(downloadInfo.value.downloadUrl, '_blank')
    showStatusMessage('Download started', 'success')
  }
}

function addBuildLog(message, type = 'info') {
  buildLogs.value.push({
    id: Date.now() + Math.random(),
    timestamp: new Date().toISOString(),
    message,
    type
  })
}

function clearLogs() {
  buildLogs.value = []
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
.payload-page {
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

.payload-layout {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-6);
  flex: 1;
  min-height: 0;
}

.config-panel {
  height: fit-content;
}

.output-panel {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  height: 100%;
}

.download-section {
  flex-shrink: 0;
}

.build-logs {
  flex: 1;
  min-height: 300px;
}

.logs-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.logs-header h3 {
  margin: 0;
}

@media (max-width: 1024px) {
  .payload-layout {
    grid-template-columns: 1fr;
  }
}
</style>