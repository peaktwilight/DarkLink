<template>
  <form @submit.prevent="submitForm" class="listener-form">
    <!-- Basic Settings -->
    <div class="form-section">
      <h3>Basic Settings</h3>
      
      <div class="form-group">
        <label for="listenerName">Name</label>
        <input 
          id="listenerName"
          v-model="form.name"
          type="text" 
          required 
          placeholder="e.g., base-http"
          class="form-input"
        >
      </div>
      
      <div class="form-group">
        <label for="payloadType">Protocol</label>
        <select id="payloadType" v-model="form.protocol" required class="form-select">
          <option value="http">HTTP</option>
          <option value="https">HTTPS</option>
          <option value="dns">DNS over HTTPS (WIP)</option>
        </select>
      </div>
    </div>

    <!-- Network Configuration -->
    <div class="form-section">
      <h3>Network Configuration</h3>
      
      <div class="form-group">
        <label for="bindHost">Bind Host</label>
        <input 
          id="bindHost"
          v-model="form.bindHost"
          type="text" 
          placeholder="0.0.0.0"
          class="form-input"
        >
        <small class="form-help">Use 0.0.0.0 to bind to all interfaces</small>
      </div>

      <div class="form-group">
        <label for="port">Port</label>
        <input 
          id="port"
          v-model.number="form.port"
          type="number" 
          min="1024" 
          max="65535"
          required
          class="form-input"
        >
        <small class="form-help">Use ports above 1024 for non-root access</small>
      </div>
    </div>

    <!-- Host Configuration -->
    <div class="form-section">
      <h3>Host Configuration</h3>
      
      <div class="form-group">
        <label for="hostInput">Hosts</label>
        <div class="input-with-button">
          <input 
            id="hostInput"
            v-model="hostInput"
            type="text" 
            placeholder="domain.com or IP address"
            class="form-input"
            @keypress="handleHostKeyPress"
          >
          <Button 
            type="button" 
            variant="secondary" 
            size="small"
            @click="addHost"
          >
            Add
          </Button>
        </div>
        
        <div v-if="form.hosts.length" class="items-list">
          <div 
            v-for="(host, index) in form.hosts" 
            :key="index"
            class="list-item"
          >
            <span>{{ host }}</span>
            <Button 
              variant="danger" 
              size="small"
              icon="x"
              @click="removeHost(index)"
            />
          </div>
        </div>
      </div>

      <div class="form-group">
        <label for="hostRotation">Host Rotation</label>
        <select id="hostRotation" v-model="form.hostRotation" class="form-select">
          <option value="round-robin">Round Robin</option>
          <option value="random">Random</option>
        </select>
        <small class="form-help">Note: Currently work in progress</small>
      </div>
    </div>

    <!-- HTTP Configuration -->
    <div class="form-section">
      <h3>HTTP Configuration</h3>
      
      <div class="form-group">
        <label for="userAgent">User Agent</label>
        <input 
          id="userAgent"
          v-model="form.userAgent"
          type="text" 
          class="form-input"
        >
      </div>

      <div class="form-group">
        <label for="headerInput">Custom Headers</label>
        <div class="input-with-button">
          <input 
            id="headerInput"
            v-model="headerInput"
            type="text" 
            placeholder="X-Header: Value"
            class="form-input"
            @keypress="handleHeaderKeyPress"
          >
          <Button 
            type="button" 
            variant="secondary" 
            size="small"
            @click="addHeader"
          >
            Add
          </Button>
        </div>
        
        <div v-if="form.headers.length" class="items-list">
          <div 
            v-for="(header, index) in form.headers" 
            :key="index"
            class="list-item"
          >
            <span>{{ header }}</span>
            <Button 
              variant="danger" 
              size="small"
              icon="x"
              @click="removeHeader(index)"
            />
          </div>
        </div>
      </div>

      <div class="form-group">
        <label for="uriInput">URIs</label>
        <div class="input-with-button">
          <input 
            id="uriInput"
            v-model="uriInput"
            type="text" 
            placeholder="/api/agent/"
            class="form-input"
            @keypress="handleUriKeyPress"
          >
          <Button 
            type="button" 
            variant="secondary" 
            size="small"
            @click="addUri"
          >
            Add
          </Button>
        </div>
        
        <div v-if="form.uris.length" class="items-list">
          <div 
            v-for="(uri, index) in form.uris" 
            :key="index"
            class="list-item"
          >
            <span>{{ uri }}</span>
            <Button 
              variant="danger" 
              size="small"
              icon="x"
              @click="removeUri(index)"
            />
          </div>
        </div>
      </div>

      <div class="form-group">
        <label for="hostHeader">Host Header</label>
        <input 
          id="hostHeader"
          v-model="form.hostHeader"
          type="text" 
          class="form-input"
        >
      </div>
    </div>

    <!-- Proxy Settings -->
    <div class="form-section">
      <h3>Proxy Settings</h3>
      
      <div class="form-group">
        <label class="checkbox-label">
          <input 
            v-model="form.enableProxy"
            type="checkbox"
          >
          Enable Proxy Connection
        </label>
      </div>

      <div v-if="form.enableProxy" class="proxy-settings">
        <div class="form-group">
          <label for="proxyType">Proxy Type</label>
          <select id="proxyType" v-model="form.proxyType" class="form-select">
            <option value="http">HTTP</option>
            <option value="https">HTTPS</option>
          </select>
        </div>

        <div class="form-group">
          <label for="proxyHost">Proxy Host</label>
          <input 
            id="proxyHost"
            v-model="form.proxyHost"
            type="text" 
            class="form-input"
          >
        </div>

        <div class="form-group">
          <label for="proxyPort">Proxy Port</label>
          <input 
            id="proxyPort"
            v-model.number="form.proxyPort"
            type="number" 
            class="form-input"
          >
        </div>

        <div class="form-group">
          <label for="proxyUsername">Username (optional)</label>
          <input 
            id="proxyUsername"
            v-model="form.proxyUsername"
            type="text" 
            class="form-input"
          >
        </div>

        <div class="form-group">
          <label for="proxyPassword">Password (optional)</label>
          <input 
            id="proxyPassword"
            v-model="form.proxyPassword"
            type="password" 
            class="form-input"
          >
        </div>
      </div>
    </div>

    <!-- Form Actions -->
    <div class="form-actions">
      <Button 
        type="submit" 
        variant="primary"
        :loading="loading"
        :disabled="!isFormValid"
      >
        Create Listener
      </Button>
      <Button 
        type="button" 
        variant="secondary"
        @click="resetForm"
      >
        Reset
      </Button>
    </div>
  </form>
</template>

<script setup>
import { ref, reactive, computed } from 'vue'
import Button from '../ui/Button.vue'

const props = defineProps({
  loading: {
    type: Boolean,
    default: false
  }
})

const emit = defineEmits(['submit'])

// Form data
const form = reactive({
  name: '',
  protocol: 'http',
  bindHost: '0.0.0.0',
  port: 8443,
  hosts: [],
  hostRotation: 'round-robin',
  userAgent: 'Mozilla/5.0 (Windows NT 6.1; WOW64) AppleWebKit/537.36',
  headers: [],
  uris: [],
  hostHeader: '',
  enableProxy: false,
  proxyType: 'http',
  proxyHost: '',
  proxyPort: null,
  proxyUsername: '',
  proxyPassword: ''
})

// Input helpers
const hostInput = ref('')
const headerInput = ref('')
const uriInput = ref('')

// Computed
const isFormValid = computed(() => {
  return form.name.trim() && form.protocol && form.port > 0
})

// Methods
function addHost() {
  const host = hostInput.value.trim()
  if (host && !form.hosts.includes(host)) {
    form.hosts.push(host)
    hostInput.value = ''
  }
}

function removeHost(index) {
  form.hosts.splice(index, 1)
}

function addHeader() {
  const header = headerInput.value.trim()
  if (header && header.includes(':') && !form.headers.includes(header)) {
    form.headers.push(header)
    headerInput.value = ''
  }
}

function removeHeader(index) {
  form.headers.splice(index, 1)
}

function addUri() {
  const uri = uriInput.value.trim()
  if (uri && !form.uris.includes(uri)) {
    form.uris.push(uri)
    uriInput.value = ''
  }
}

function removeUri(index) {
  form.uris.splice(index, 1)
}

function handleHostKeyPress(event) {
  if (event.key === 'Enter') {
    event.preventDefault()
    addHost()
  }
}

function handleHeaderKeyPress(event) {
  if (event.key === 'Enter') {
    event.preventDefault()
    addHeader()
  }
}

function handleUriKeyPress(event) {
  if (event.key === 'Enter') {
    event.preventDefault()
    addUri()
  }
}

function submitForm() {
  if (!isFormValid.value) return
  
  // Convert headers array to map format expected by Go backend
  const headersMap = {}
  form.headers.forEach(header => {
    const [key, ...valueParts] = header.split(':')
    if (key && valueParts.length > 0) {
      headersMap[key.trim()] = valueParts.join(':').trim()
    }
  })
  
  const config = {
    name: form.name,
    protocol: form.protocol,
    bindHost: form.bindHost || '0.0.0.0',
    port: form.port,
    uris: form.uris,           // Keep as array
    headers: headersMap,       // Convert to map/object
    userAgent: form.userAgent,
    hostRotation: form.hostRotation,
    hosts: form.hosts,         // Keep as array
    // Add proxy config as nested object if enabled
    ...(form.enableProxy && {
      proxy: {
        type: form.proxyType,
        host: form.proxyHost,
        port: form.proxyPort,
        username: form.proxyUsername || undefined,
        password: form.proxyPassword || undefined
      }
    })
  }
  
  // Debug: Log the exact config being sent
  console.log('🐛 Submitting listener config:', JSON.stringify(config, null, 2))
  
  emit('submit', config)
}

function resetForm() {
  Object.assign(form, {
    name: '',
    protocol: 'http',
    bindHost: '0.0.0.0',
    port: 8443,
    hosts: [],
    hostRotation: 'round-robin',
    userAgent: 'Mozilla/5.0 (Windows NT 6.1; WOW64) AppleWebKit/537.36',
    headers: [],
    uris: [],
    hostHeader: '',
    enableProxy: false,
    proxyType: 'http',
    proxyHost: '',
    proxyPort: null,
    proxyUsername: '',
    proxyPassword: ''
  })
  
  hostInput.value = ''
  headerInput.value = ''
  uriInput.value = ''
}
</script>

<style scoped>
.listener-form {
  display: flex;
  flex-direction: column;
  gap: var(--space-6);
}

.form-section {
  border: 1px solid var(--border-color);
  border-radius: var(--radius);
  padding: var(--space-4);
}

.form-section h3 {
  margin: 0 0 var(--space-4) 0;
  color: var(--accent-color);
  font-size: 1rem;
  font-weight: 600;
}

.form-group {
  margin-bottom: var(--space-4);
}

.form-group:last-child {
  margin-bottom: 0;
}

.form-group label {
  display: block;
  margin-bottom: var(--space-2);
  font-weight: 500;
  color: var(--text-color);
}

.form-input,
.form-select {
  width: 100%;
  padding: var(--space-2) var(--space-3);
  background: var(--secondary-bg);
  border: 1px solid var(--border-color);
  border-radius: var(--radius);
  color: var(--text-color);
  font-size: 0.875rem;
  transition: all 0.2s ease;
}

.form-input:focus,
.form-select:focus {
  outline: none;
  border-color: var(--accent-color);
  box-shadow: 0 0 0 2px rgba(88, 166, 255, 0.3);
}

.form-help {
  display: block;
  margin-top: var(--space-1);
  font-size: 0.75rem;
  color: var(--text-secondary);
}

.input-with-button {
  display: flex;
  gap: var(--space-2);
  align-items: center;
}

.input-with-button .form-input {
  flex: 1;
}

.items-list {
  margin-top: var(--space-3);
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.list-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: var(--space-2) var(--space-3);
  background: var(--tertiary-bg);
  border-radius: var(--radius-sm);
  font-family: var(--font-mono);
  font-size: 0.875rem;
}

.checkbox-label {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  cursor: pointer;
  font-weight: 500;
}

.checkbox-label input[type="checkbox"] {
  width: auto;
  margin: 0;
}

.proxy-settings {
  margin-top: var(--space-4);
  padding-top: var(--space-4);
  border-top: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.form-actions {
  display: flex;
  gap: var(--space-3);
  justify-content: flex-end;
  padding-top: var(--space-4);
  border-top: 1px solid var(--border-color);
}
</style>